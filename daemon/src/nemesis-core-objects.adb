with Ada.Directories;
with Ada.Streams;
with Ada.Streams.Stream_IO;
with Ada.Strings;
with Ada.Strings.Fixed;
with GNAT.OS_Lib;
with Interfaces.C;

package body Nemesis.Core.Objects with SPARK_Mode => Off is
   use type Ada.Streams.Stream_Element_Count;
   use type GNAT.OS_Lib.File_Descriptor;
   use type Interfaces.C.int;

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

   function Path_Of (Root : String; Object_Digest : Digest_Hex) return String is
   begin
      return
        Root & "/objects/sha256/" & Object_Digest (1 .. 2) & "/"
        & Object_Digest (3 .. Object_Digest'Last);
   end Path_Of;

   procedure Get
     (Root          : String;
      Object_Digest : Digest_Hex;
      Result        : out Load_Result;
      Data          : out Ada.Strings.Unbounded.Unbounded_String)
   is
      package SIO renames Ada.Streams.Stream_IO;
      Input : SIO.File_Type;
      Path  : constant String := Path_Of (Root, Object_Digest);
   begin
      Data := Ada.Strings.Unbounded.Null_Unbounded_String;
      if not Is_Hex (Object_Digest) then
         Result := Integrity_Failed;
         return;
      end if;
      if not Ada.Directories.Exists (Path) then
         Result := Object_Not_Found;
         return;
      end if;

      SIO.Open (Input, SIO.In_File, Path);
      declare
         Length : constant Ada.Streams.Stream_Element_Count :=
           Ada.Streams.Stream_Element_Count (SIO.Size (Input));
         Bytes : Ada.Streams.Stream_Element_Array (1 .. Length);
         Last  : Ada.Streams.Stream_Element_Offset;
         Value : String (1 .. Integer (Length));
      begin
         if Length > 0 then
            SIO.Read (Input, Bytes, Last);
            if Last /= Bytes'Last then
               SIO.Close (Input);
               Result := Load_Failed;
               return;
            end if;
            for Index in Bytes'Range loop
               Value (Integer (Index)) := Character'Val (Bytes (Index));
            end loop;
         end if;
         SIO.Close (Input);
         Data := Ada.Strings.Unbounded.To_Unbounded_String (Value);
         if Digest (Value) = Object_Digest then
            Result := Loaded_Valid;
         else
            Result := Integrity_Failed;
         end if;
      end;
   exception
      when others =>
         if SIO.Is_Open (Input) then
            SIO.Close (Input);
         end if;
         Data := Ada.Strings.Unbounded.Null_Unbounded_String;
         Result := Load_Failed;
   end Get;

   procedure Put
     (Root          : String;
      Data          : String;
      Result        : out Store_Result;
      Object_Digest : out Digest_Hex)
   is
      Existing_Result : Load_Result;
      Existing_Data   : Ada.Strings.Unbounded.Unbounded_String;
      Final_Path      : String := Path_Of (Root, Digest (Data));
      Directory       : constant String :=
        Root & "/objects/sha256/" & Digest (Data) (1 .. 2);
      Pid_Image       : constant String :=
        Ada.Strings.Fixed.Trim
          (Integer'Image
             (GNAT.OS_Lib.Pid_To_Integer (GNAT.OS_Lib.Current_Process_Id)),
           Ada.Strings.Both);
      Temp_Path       : constant String := Final_Path & ".tmp-" & Pid_Image;
      FD              : GNAT.OS_Lib.File_Descriptor := GNAT.OS_Lib.Invalid_FD;
      Close_OK        : Boolean := False;
      Rename_OK       : Boolean := False;
   begin
      Object_Digest := Digest (Data);
      Final_Path := Path_Of (Root, Object_Digest);
      Ada.Directories.Create_Path (Directory);

      if Ada.Directories.Exists (Final_Path) then
         Get (Root, Object_Digest, Existing_Result, Existing_Data);
         if Existing_Result = Loaded_Valid
           and then Ada.Strings.Unbounded.To_String (Existing_Data) = Data
         then
            Result := Already_Present;
         else
            Result := Collision_Detected;
         end if;
         return;
      end if;

      if Ada.Directories.Exists (Temp_Path) then
         Ada.Directories.Delete_File (Temp_Path);
      end if;
      FD := GNAT.OS_Lib.Create_New_File (Temp_Path, GNAT.OS_Lib.Binary);
      if FD = GNAT.OS_Lib.Invalid_FD then
         Result := Store_Write_Failed;
         return;
      end if;

      declare
         Buffer : aliased String := Data;
         Written : Integer;
      begin
         if Buffer'Length = 0 then
            Written := 0;
         else
            Written := GNAT.OS_Lib.Write (FD, Buffer'Address, Buffer'Length);
         end if;
         if Written /= Buffer'Length then
            GNAT.OS_Lib.Close (FD, Close_OK);
            Result := Store_Write_Failed;
            return;
         end if;
      end;

      if Fsync (Interfaces.C.int (FD)) /= 0 then
         GNAT.OS_Lib.Close (FD, Close_OK);
         Result := Store_Sync_Failed;
         return;
      end if;

      GNAT.OS_Lib.Close (FD, Close_OK);
      if not Close_OK then
         Result := Store_Close_Failed;
         return;
      end if;

      GNAT.OS_Lib.Rename_File (Temp_Path, Final_Path, Rename_OK);
      if not Rename_OK then
         Result := Store_Rename_Failed;
         return;
      end if;
      Result := Stored;
   exception
      when others =>
         if FD /= GNAT.OS_Lib.Invalid_FD then
            GNAT.OS_Lib.Close (FD, Close_OK);
         end if;
         Result := Store_Write_Failed;
   end Put;

end Nemesis.Core.Objects;
