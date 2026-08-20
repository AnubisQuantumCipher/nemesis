with Ada.Directories;
with Ada.Streams;
with Ada.Streams.Stream_IO;
with Ada.Strings;
with Ada.Strings.Fixed;
with GNAT.OS_Lib;
with GNAT.SHA256;
with Interfaces.C;

package body Nemesis.Core.Ledger with SPARK_Mode => Off is
   use type Ada.Streams.Stream_Element;
   use type Ada.Streams.Stream_Element_Count;
   use type GNAT.OS_Lib.File_Descriptor;
   use type Interfaces.C.int;
   Schema_Tag    : constant String := "NEMESIS_LEDGER_V1";
   Core_Length   : constant Positive := 239;
   Line_Length   : constant Positive := 304;
   Record_Length : constant Positive := 305;

   function Fsync (FD : Interfaces.C.int) return Interfaces.C.int
   with Import, Convention => C, External_Name => "fsync";

   function Digest (Value : String) return Digest_Hex is
      Raw : constant GNAT.SHA256.Message_Digest := GNAT.SHA256.Digest (Value);
   begin
      return Digest_Hex (Raw);
   end Digest;

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
     (Value : Long_Long_Integer; Width : Positive) return String
   is
      Image : constant String :=
        Ada.Strings.Fixed.Trim (Long_Long_Integer'Image (Value), Ada.Strings.Both);
      Result : String (1 .. Width) := [others => '0'];
   begin
      if Image'Length > Width then
         raise Constraint_Error with "ledger decimal field exceeds width";
      end if;
      Result (Width - Image'Length + 1 .. Width) := Image;
      return Result;
   end Padded_Decimal;

   function Canonical_Core
     (Sequence       : Sequence_Number;
      Kind           : Event_Kind;
      State          : Mission_State;
      Source_Digest  : Digest_Hex;
      Payload_Digest : Digest_Hex;
      Previous       : Digest_Hex) return String
   is
      Result : constant String :=
        Schema_Tag & "|"
        & Padded_Decimal (Long_Long_Integer (Sequence), 20) & "|"
        & Padded_Decimal (Long_Long_Integer (Event_Kind'Pos (Kind)), 2) & "|"
        & Padded_Decimal (Long_Long_Integer (Mission_State'Pos (State)), 2) & "|"
        & Source_Digest & "|"
        & Payload_Digest & "|"
        & Previous;
   begin
      pragma Assert (Result'Length = Core_Length);
      return Result;
   end Canonical_Core;

   function To_String
     (Data  : Ada.Streams.Stream_Element_Array;
      First : Ada.Streams.Stream_Element_Offset;
      Last  : Ada.Streams.Stream_Element_Offset) return String
   is
      Result : String (1 .. Integer (Last - First + 1));
      Cursor : Positive := Result'First;
   begin
      for Index in First .. Last loop
         Result (Cursor) := Character'Val (Data (Index));
         Cursor := Cursor + 1;
      end loop;
      return Result;
   end To_String;

   procedure Recover (Path : String; Result : out Recovery_Result) is
      package SIO renames Ada.Streams.Stream_IO;
      Input : SIO.File_Type;
   begin
      Result :=
        (Status   => Empty,
         Sequence => Sequence_Number'First,
         State    => Draft,
         Head     => Zero_Digest);

      if not Ada.Directories.Exists (Path) then
         return;
      end if;

      SIO.Open (Input, SIO.In_File, Path);
      declare
         Length : constant Ada.Streams.Stream_Element_Count :=
           Ada.Streams.Stream_Element_Count (SIO.Size (Input));
      begin
         if Length = 0 then
            SIO.Close (Input);
            return;
         end if;

         declare
            Data : Ada.Streams.Stream_Element_Array (1 .. Length);
            Last : Ada.Streams.Stream_Element_Offset;
            Complete_Records : constant Natural :=
              Natural (Length / Ada.Streams.Stream_Element_Count (Record_Length));
            Remainder : constant Natural :=
              Natural (Length mod Ada.Streams.Stream_Element_Count (Record_Length));
            Expected_Sequence : Sequence_Number := Sequence_Number'First;
            Expected_Previous : Digest_Hex := Zero_Digest;
         begin
            SIO.Read (Input, Data, Last);
            SIO.Close (Input);
            if Last /= Data'Last then
               Result.Status := Read_Failed;
               return;
            end if;

            for Record_Index in 0 .. Complete_Records - 1 loop
               declare
                  First : constant Ada.Streams.Stream_Element_Offset :=
                    Ada.Streams.Stream_Element_Offset
                      (Record_Index * Record_Length + 1);
                  Line_Last : constant Ada.Streams.Stream_Element_Offset :=
                    First + Ada.Streams.Stream_Element_Offset (Line_Length - 1);
                  Newline : constant Ada.Streams.Stream_Element_Offset :=
                    Line_Last + 1;
                  Line : constant String := To_String (Data, First, Line_Last);
                  Sequence : Sequence_Number;
                  Kind_Code : Integer;
                  State_Value : Mission_State;
                  Source_Value : Digest_Hex;
                  Payload_Value : Digest_Hex;
                  Previous_Value : Digest_Hex;
                  Stored_Hash : Digest_Hex;
                  Calculated_Hash : Digest_Hex;
               begin
                  if Data (Newline) /=
                    Ada.Streams.Stream_Element (Character'Pos (ASCII.LF))
                  then
                     Result.Status := Corrupt;
                     return;
                  end if;

                  if Line (1 .. Schema_Tag'Length) /= Schema_Tag then
                     if Record_Index = 0 then
                        Result.Status := Unsupported_Version;
                     else
                        Result.Status := Corrupt;
                     end if;
                     return;
                  end if;

                  if Line (18) /= '|'
                    or else Line (39) /= '|'
                    or else Line (42) /= '|'
                    or else Line (45) /= '|'
                    or else Line (110) /= '|'
                    or else Line (175) /= '|'
                    or else Line (240) /= '|'
                  then
                     Result.Status := Corrupt;
                     return;
                  end if;

                  Sequence := Sequence_Number'Value (Line (19 .. 38));
                  Kind_Code := Integer'Value (Line (40 .. 41));
                  State_Value :=
                    Decode_State (State_Code'Value (Line (43 .. 44)));
                  Source_Value := Digest_Hex (Line (46 .. 109));
                  Payload_Value := Digest_Hex (Line (111 .. 174));
                  Previous_Value := Digest_Hex (Line (176 .. 239));
                  Stored_Hash := Digest_Hex (Line (241 .. Line_Length));

                  if Kind_Code < Event_Kind'Pos (Event_Kind'First)
                    or else Kind_Code > Event_Kind'Pos (Event_Kind'Last)
                    or else not Is_Hex (Source_Value)
                    or else not Is_Hex (Payload_Value)
                    or else not Is_Hex (Previous_Value)
                    or else not Is_Hex (Stored_Hash)
                  then
                     Result.Status := Corrupt;
                     return;
                  end if;

                  if Expected_Sequence = Sequence_Number'Last then
                     Result.Status := Corrupt;
                     return;
                  end if;
                  Expected_Sequence := Next (Expected_Sequence);
                  if Sequence /= Expected_Sequence
                    or else Previous_Value /= Expected_Previous
                  then
                     Result.Status := Corrupt;
                     return;
                  end if;

                  Calculated_Hash := Digest (Line (1 .. Core_Length));
                  if Stored_Hash /= Calculated_Hash then
                     Result.Status := Corrupt;
                     return;
                  end if;

                  Result.Sequence := Sequence;
                  Result.State := State_Value;
                  Result.Head := Stored_Hash;
                  Expected_Previous := Stored_Hash;
               exception
                  when Constraint_Error =>
                     Result.Status := Corrupt;
                     return;
               end;
            end loop;

            if Remainder = 0 then
               Result.Status := Recovered_Valid;
            else
               Result.Status := Truncated_Tail;
            end if;
         end;
      end;
   exception
      when others =>
         if SIO.Is_Open (Input) then
            SIO.Close (Input);
         end if;
         Result.Status := Read_Failed;
   end Recover;

   procedure Append
     (Path           : String;
      Kind           : Event_Kind;
      State          : Mission_State;
      Source_Digest  : Digest_Hex;
      Payload_Digest : Digest_Hex;
      Result         : out Append_Result;
      Event_Digest   : out Digest_Hex)
   is
      Current : Recovery_Result;
      Previous : Digest_Hex;
      Sequence : Sequence_Number;
      FD : GNAT.OS_Lib.File_Descriptor := GNAT.OS_Lib.Invalid_FD;
      Close_OK : Boolean := False;
   begin
      Result := Invalid_Input;
      Event_Digest := Zero_Digest;
      if not Is_Hex (Source_Digest) or else not Is_Hex (Payload_Digest) then
         return;
      end if;

      Recover (Path, Current);
      case Current.Status is
         when Empty =>
            Previous := Zero_Digest;
            Sequence := Sequence_Number'First;
         when Recovered_Valid =>
            Previous := Current.Head;
            Sequence := Current.Sequence;
         when others =>
            Result := Invalid_Existing_Ledger;
            return;
      end case;

      if Sequence = Sequence_Number'Last then
         Result := Sequence_Exhausted;
         return;
      end if;
      Sequence := Next (Sequence);

      declare
         Core : constant String :=
           Canonical_Core
             (Sequence, Kind, State, Source_Digest, Payload_Digest, Previous);
         Hash : constant Digest_Hex := Digest (Core);
         Data : aliased String := Core & "|" & Hash & ASCII.LF;
         Written : Integer;
         Synced : Interfaces.C.int;
      begin
         pragma Assert (Data'Length = Record_Length);
         if Ada.Directories.Exists (Path) then
            FD := GNAT.OS_Lib.Open_Append (Path, GNAT.OS_Lib.Binary);
         else
            FD := GNAT.OS_Lib.Create_File (Path, GNAT.OS_Lib.Binary);
         end if;
         if FD = GNAT.OS_Lib.Invalid_FD then
            Result := Write_Failed;
            return;
         end if;

         Written := GNAT.OS_Lib.Write (FD, Data'Address, Data'Length);
         if Written /= Data'Length then
            GNAT.OS_Lib.Close (FD, Close_OK);
            Result := Write_Failed;
            return;
         end if;

         Synced := Fsync (Interfaces.C.int (FD));
         if Synced /= 0 then
            GNAT.OS_Lib.Close (FD, Close_OK);
            Result := Sync_Failed;
            return;
         end if;

         GNAT.OS_Lib.Close (FD, Close_OK);
         if not Close_OK then
            Result := Close_Failed;
            return;
         end if;

         Event_Digest := Hash;
         Result := Committed;
      end;
   end Append;

end Nemesis.Core.Ledger;
