with Ada.Directories;
with Ada.Streams;
with Ada.Streams.Stream_IO;
with Ada.Strings;
with Ada.Strings.Fixed;
with GNAT.OS_Lib;
with Interfaces.C;

package body Nemesis.Core.Checkpoints with SPARK_Mode => Off is
   use type Ada.Streams.Stream_Element;
   use type Ada.Streams.Stream_Element_Count;
   use type Ada.Streams.Stream_IO.Count;
   use type GNAT.OS_Lib.File_Descriptor;
   use type Interfaces.C.int;

   Schema_Tag  : constant String := "NEMESIS_CHECKPOINT_V1";
   Core_Length : constant Positive := 175;
   Line_Length : constant Positive := 240;
   File_Length : constant Positive := 241;

   function Fsync (FD : Interfaces.C.int) return Interfaces.C.int
   with Import, Convention => C, External_Name => "fsync";

   function Is_Hex (Value : Digest_Hex) return Boolean is
   begin
      for Item of Value loop
         if not (Item in '0' .. '9' | 'a' .. 'f') then
            return False;
         end if;
      end loop;
      return True;
   end Is_Hex;

   function Padded_Decimal
     (Number : Long_Long_Integer; Width : Positive) return String
   is
      Image : constant String :=
        Ada.Strings.Fixed.Trim
          (Long_Long_Integer'Image (Number), Ada.Strings.Both);
      Output : String (1 .. Width) := [others => '0'];
   begin
      if Image'Length > Width then
         raise Constraint_Error with "checkpoint decimal field exceeds width";
      end if;
      Output (Width - Image'Length + 1 .. Width) := Image;
      return Output;
   end Padded_Decimal;

   function Canonical_Core (Value : Checkpoint_Record) return String is
      Output : constant String :=
        Schema_Tag & "|"
        & Padded_Decimal (Long_Long_Integer (Value.Sequence), 20) & "|"
        & Padded_Decimal
            (Long_Long_Integer (Mission_State'Pos (Value.State)), 2) & "|"
        & Value.Ledger_Head & "|" & Value.Source_Digest;
   begin
      pragma Assert (Output'Length = Core_Length);
      return Output;
   end Canonical_Core;

   function To_String
     (Data : Ada.Streams.Stream_Element_Array) return String
   is
      Output : String (1 .. Data'Length);
      Cursor : Positive := Output'First;
   begin
      for Item of Data loop
         Output (Cursor) := Character'Val (Item);
         Cursor := Cursor + 1;
      end loop;
      return Output;
   end To_String;

   procedure Read_Checkpoint
     (Path   : String;
      Result : out Checkpoint_Read_Result;
      Value  : out Checkpoint_Record)
   is
      package SIO renames Ada.Streams.Stream_IO;
      Input : SIO.File_Type;
   begin
      Value :=
        (Sequence      => Sequence_Number'First,
         State         => Draft,
         Ledger_Head   => Zero_Digest,
         Source_Digest => Zero_Digest);
      if not Ada.Directories.Exists (Path) then
         Result := Checkpoint_Missing;
         return;
      end if;

      SIO.Open (Input, SIO.In_File, Path);
      if SIO.Size (Input) /= Ada.Streams.Stream_IO.Count (File_Length) then
         SIO.Close (Input);
         Result := Checkpoint_Corrupt;
         return;
      end if;
      declare
         Data : Ada.Streams.Stream_Element_Array
           (1 .. Ada.Streams.Stream_Element_Offset (File_Length));
         Last : Ada.Streams.Stream_Element_Offset;
      begin
         SIO.Read (Input, Data, Last);
         SIO.Close (Input);
         if Last /= Data'Last
           or else Data (Data'Last) /=
             Ada.Streams.Stream_Element (Character'Pos (ASCII.LF))
         then
            Result := Checkpoint_Corrupt;
            return;
         end if;

         declare
            Line : constant String :=
              To_String (Data (Data'First .. Data'Last - 1));
            Stored_Hash : Digest_Hex;
         begin
            if Line (1 .. Schema_Tag'Length) /= Schema_Tag then
               Result := Checkpoint_Unsupported_Version;
               return;
            end if;
            if Line (22) /= '|'
              or else Line (43) /= '|'
              or else Line (46) /= '|'
              or else Line (111) /= '|'
              or else Line (176) /= '|'
            then
               Result := Checkpoint_Corrupt;
               return;
            end if;

            Value.Sequence := Sequence_Number'Value (Line (23 .. 42));
            Value.State := Decode_State (State_Code'Value (Line (44 .. 45)));
            Value.Ledger_Head := Digest_Hex (Line (47 .. 110));
            Value.Source_Digest := Digest_Hex (Line (112 .. 175));
            Stored_Hash := Digest_Hex (Line (177 .. Line_Length));
            if not Is_Hex (Value.Ledger_Head)
              or else not Is_Hex (Value.Source_Digest)
              or else not Is_Hex (Stored_Hash)
              or else Stored_Hash /= Digest (Line (1 .. Core_Length))
            then
               Result := Checkpoint_Corrupt;
               return;
            end if;
            Result := Checkpoint_Valid;
         exception
            when Constraint_Error =>
               Result := Checkpoint_Corrupt;
         end;
      end;
   exception
      when others =>
         if SIO.Is_Open (Input) then
            SIO.Close (Input);
         end if;
         Result := Checkpoint_Read_Failed;
   end Read_Checkpoint;

   procedure Write_Checkpoint
     (Path              : String;
      Value             : Checkpoint_Record;
      Result            : out Checkpoint_Write_Result;
      Checkpoint_Digest : out Digest_Hex)
   is
      Directory : constant String := Ada.Directories.Containing_Directory (Path);
      Pid_Image : constant String :=
        Ada.Strings.Fixed.Trim
          (Integer'Image
             (GNAT.OS_Lib.Pid_To_Integer (GNAT.OS_Lib.Current_Process_Id)),
           Ada.Strings.Both);
      Temp_Path : constant String := Path & ".tmp-" & Pid_Image;
      FD : GNAT.OS_Lib.File_Descriptor := GNAT.OS_Lib.Invalid_FD;
      Close_OK  : Boolean := False;
      Rename_OK : Boolean := False;
   begin
      Checkpoint_Digest := Zero_Digest;
      if not Is_Hex (Value.Ledger_Head)
        or else not Is_Hex (Value.Source_Digest)
      then
         Result := Checkpoint_Invalid_Input;
         return;
      end if;

      if not Ada.Directories.Exists (Directory) then
         Ada.Directories.Create_Path (Directory);
      end if;
      if Ada.Directories.Exists (Temp_Path) then
         Ada.Directories.Delete_File (Temp_Path);
      end if;

      declare
         Core : constant String := Canonical_Core (Value);
         Hash : constant Digest_Hex := Digest (Core);
         Data : aliased String := Core & "|" & Hash & ASCII.LF;
         Written : Integer;
      begin
         pragma Assert (Data'Length = File_Length);
         FD := GNAT.OS_Lib.Create_New_File (Temp_Path, GNAT.OS_Lib.Binary);
         if FD = GNAT.OS_Lib.Invalid_FD then
            Result := Checkpoint_Write_Failed;
            return;
         end if;
         Written := GNAT.OS_Lib.Write (FD, Data'Address, Data'Length);
         if Written /= Data'Length then
            GNAT.OS_Lib.Close (FD, Close_OK);
            Result := Checkpoint_Write_Failed;
            return;
         end if;
         if Fsync (Interfaces.C.int (FD)) /= 0 then
            GNAT.OS_Lib.Close (FD, Close_OK);
            Result := Checkpoint_Sync_Failed;
            return;
         end if;
         GNAT.OS_Lib.Close (FD, Close_OK);
         if not Close_OK then
            Result := Checkpoint_Close_Failed;
            return;
         end if;
         GNAT.OS_Lib.Rename_File (Temp_Path, Path, Rename_OK);
         if not Rename_OK then
            Result := Checkpoint_Rename_Failed;
            return;
         end if;
         Checkpoint_Digest := Hash;
         Result := Checkpoint_Committed;
      end;
   exception
      when others =>
         if FD /= GNAT.OS_Lib.Invalid_FD then
            GNAT.OS_Lib.Close (FD, Close_OK);
         end if;
         Result := Checkpoint_Write_Failed;
   end Write_Checkpoint;

end Nemesis.Core.Checkpoints;
