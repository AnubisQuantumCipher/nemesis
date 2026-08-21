with Ada.Directories;
with Ada.Strings.Unbounded;
with Interfaces.C.Strings;

package body Nemesis.Core.Paths with SPARK_Mode => Off is
   use Ada.Strings.Unbounded;
   use type Ada.Directories.File_Kind;
   use type Interfaces.C.Strings.chars_ptr;

   function C_Realpath
     (Path     : Interfaces.C.Strings.chars_ptr;
      Resolved : Interfaces.C.Strings.chars_ptr)
      return Interfaces.C.Strings.chars_ptr
   with Import, Convention => C, External_Name => "realpath";

   function Is_Absolute (Path : String) return Boolean is
     (Path'Length > 0 and then Path (Path'First) = '/');

   function Has_Unsafe_Segment (Path : String) return Boolean is
      Segment_Start : Positive := Path'First;
   begin
      for Index in Path'Range loop
         if Path (Index) = ASCII.NUL then
            return True;
         elsif Path (Index) = '/' then
            if Index > Segment_Start then
               declare
                  Segment : constant String := Path (Segment_Start .. Index - 1);
               begin
                  if Segment in "." | ".." then
                     return True;
                  end if;
               end;
            end if;
            Segment_Start := Index + 1;
         end if;
      end loop;
      if Segment_Start <= Path'Last then
         declare
            Segment : constant String := Path (Segment_Start .. Path'Last);
         begin
            return Segment in "." | "..";
         end;
      end if;
      return False;
   end Has_Unsafe_Segment;

   procedure Resolve
     (Path    : String;
      Success : out Boolean;
      Value   : out Unbounded_String)
   is
      Input  : Interfaces.C.Strings.chars_ptr :=
        Interfaces.C.Strings.New_String (Path);
      Output : Interfaces.C.Strings.chars_ptr;
   begin
      Output := C_Realpath (Input, Interfaces.C.Strings.Null_Ptr);
      Interfaces.C.Strings.Free (Input);
      if Output = Interfaces.C.Strings.Null_Ptr then
         Value := Null_Unbounded_String;
         Success := False;
         return;
      end if;
      Value := To_Unbounded_String (Interfaces.C.Strings.Value (Output));
      Interfaces.C.Strings.Free (Output);
      Success := True;
   exception
      when others =>
         if Input /= Interfaces.C.Strings.Null_Ptr then
            Interfaces.C.Strings.Free (Input);
         end if;
         if Output /= Interfaces.C.Strings.Null_Ptr then
            Interfaces.C.Strings.Free (Output);
         end if;
         Value := Null_Unbounded_String;
         Success := False;
   end Resolve;

   function Within (Root : String; Candidate : String) return Boolean is
   begin
      if Candidate = Root then
         return True;
      end if;
      return
        Candidate'Length > Root'Length
        and then Candidate (Candidate'First .. Candidate'First + Root'Length - 1) =
          Root
        and then Candidate (Candidate'First + Root'Length) = '/';
   end Within;

   function Authorize_Existing
     (Canonical_Root : String; Candidate : String) return Path_Decision
   is
      Root_Value      : Unbounded_String;
      Candidate_Value : Unbounded_String;
      Root_OK         : Boolean;
      Candidate_OK    : Boolean;
   begin
      if not Is_Absolute (Canonical_Root)
        or else not Is_Absolute (Candidate)
      then
         return Path_Not_Absolute;
      end if;
      if Has_Unsafe_Segment (Canonical_Root)
        or else Has_Unsafe_Segment (Candidate)
      then
         return Path_Traversal;
      end if;

      Resolve (Canonical_Root, Root_OK, Root_Value);
      if not Root_OK
        or else not Ada.Directories.Exists (To_String (Root_Value))
        or else Ada.Directories.Kind (To_String (Root_Value)) /=
          Ada.Directories.Directory
      then
         return Path_Invalid_Root;
      end if;
      if not Ada.Directories.Exists (Candidate) then
         return Path_Not_Found;
      end if;
      Resolve (Candidate, Candidate_OK, Candidate_Value);
      if not Candidate_OK then
         return Path_Error;
      end if;

      if not Within (To_String (Root_Value), To_String (Candidate_Value)) then
         if Within (To_String (Root_Value), Candidate) then
            return Path_Symlink_Escape;
         else
            return Path_Outside_Root;
         end if;
      end if;
      if Ada.Directories.Kind (To_String (Candidate_Value)) /=
        Ada.Directories.Ordinary_File
      then
         return Path_Special_File;
      end if;
      return Path_Authorized;
   exception
      when others =>
         return Path_Error;
   end Authorize_Existing;

   function Authorize_Create
     (Canonical_Root : String; Candidate : String) return Path_Decision
   is
      Root_Value   : Unbounded_String;
      Parent_Value : Unbounded_String;
      Root_OK      : Boolean;
      Parent_OK    : Boolean;
      Parent       : Unbounded_String;
   begin
      if not Is_Absolute (Canonical_Root)
        or else not Is_Absolute (Candidate)
      then
         return Path_Not_Absolute;
      end if;
      if Has_Unsafe_Segment (Canonical_Root)
        or else Has_Unsafe_Segment (Candidate)
      then
         return Path_Traversal;
      end if;
      if Ada.Directories.Exists (Candidate) then
         return Path_Already_Exists;
      end if;

      Resolve (Canonical_Root, Root_OK, Root_Value);
      if not Root_OK
        or else Ada.Directories.Kind (To_String (Root_Value)) /=
          Ada.Directories.Directory
      then
         return Path_Invalid_Root;
      end if;
      Parent :=
        To_Unbounded_String (Ada.Directories.Containing_Directory (Candidate));
      if not Ada.Directories.Exists (To_String (Parent)) then
         return Path_Missing_Parent;
      end if;
      Resolve (To_String (Parent), Parent_OK, Parent_Value);
      if not Parent_OK then
         return Path_Error;
      end if;

      if not Within (To_String (Root_Value), Candidate) then
         return Path_Outside_Root;
      end if;
      if not Within (To_String (Root_Value), To_String (Parent_Value)) then
         return Path_Symlink_Escape;
      end if;
      return Path_Authorized;
   exception
      when others =>
         return Path_Error;
   end Authorize_Create;

end Nemesis.Core.Paths;
