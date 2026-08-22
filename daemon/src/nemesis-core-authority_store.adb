with Ada.Directories;
with Ada.Strings;
with Ada.Strings.Fixed;
with GNAT.OS_Lib;
with Interfaces.C;

package body Nemesis.Core.Authority_Store with SPARK_Mode => Off is
   use type GNAT.OS_Lib.File_Descriptor;
   use type Interfaces.C.int;

   Grant_Tag : constant String := "NEMESIS_GRANT_V1";
   Approval_Tag : constant String := "NEMESIS_APPROVAL_V1";

   --  NEMESIS_GRANT_V1|id(26)|mission(26)|subject(26)|resource(1)|ops(10)
   --  |scope(64)|expires(20)|maximum_bytes(10)|status(1)
   Grant_Record_Length : constant := 209;

   --  NEMESIS_APPROVAL_V1|id(26)|mission(26)|action_digest(64)|expires(20)
   --  |status(1)
   Approval_Record_Length : constant := 161;

   --  macOS fsync() does not flush to stable storage; F_FULLFSYNC does.
   F_FULLFSYNC : constant Interfaces.C.int := 51;
   O_RDONLY : constant Interfaces.C.int := 0;

   function Fcntl (FD : Interfaces.C.int; Cmd : Interfaces.C.int)
     return Interfaces.C.int
   with Import, Convention => C, External_Name => "fcntl";

   function C_Open (Path : Interfaces.C.char_array; Flags : Interfaces.C.int)
     return Interfaces.C.int
   with Import, Convention => C, External_Name => "open";

   function C_Close (FD : Interfaces.C.int) return Interfaces.C.int
   with Import, Convention => C, External_Name => "close";

   procedure Full_Sync_Directory (Dir : String) is
      FD : Interfaces.C.int;
      Ignore : Interfaces.C.int;
   begin
      FD := C_Open (Interfaces.C.To_C (Dir), O_RDONLY);
      if FD >= 0 then
         Ignore := Fcntl (FD, F_FULLFSYNC);
         Ignore := C_Close (FD);
      end if;
   end Full_Sync_Directory;

   function Mission_Directory (Home : String; Id : Mission_Id) return String is
     (Home & "/missions/" & String (Id));

   function Grant_Path (Home : String; Id : Mission_Id) return String is
     (Mission_Directory (Home, Id) & "/parent.grant");

   function Approval_Path
     (Home : String; Id : Mission_Id; Action_Digest : Digest_256)
      return String
   is
     (Mission_Directory (Home, Id) & "/approval-" & String (Action_Digest)
      & ".apr");

   function Is_Hex (Value : String) return Boolean is
   begin
      if Value'Length /= Digest_256'Length then
         return False;
      end if;
      for Item of Value loop
         if not (Item in '0' .. '9' | 'a' .. 'f') then
            return False;
         end if;
      end loop;
      return True;
   end Is_Hex;

   function Is_Identifier (Value : String) return Boolean is
   begin
      if Value'Length /= 26 then
         return False;
      end if;
      for Item of Value loop
         if not (Item in 'a' .. 'z' | '0' .. '9' | '_') then
            return False;
         end if;
      end loop;
      return True;
   end Is_Identifier;

   function Is_Digits (Value : String) return Boolean is
   begin
      for Item of Value loop
         if Item not in '0' .. '9' then
            return False;
         end if;
      end loop;
      return True;
   end Is_Digits;

   function Sequence_Text (Value : Sequence_Number) return String is
     (Ada.Strings.Fixed.Tail
        (Ada.Strings.Fixed.Trim (Sequence_Number'Image (Value),
                                 Ada.Strings.Both),
         20, '0'));

   function Byte_Text (Value : Natural) return String is
     (Ada.Strings.Fixed.Tail
        (Ada.Strings.Fixed.Trim (Natural'Image (Value), Ada.Strings.Both),
         10, '0'));

   function Resource_Text (Value : Resource_Kind) return String is
     ([1 => Character'Val (Character'Pos ('0') + Resource_Kind'Pos (Value))]);

   function Operations_Text (Value : Operation_Set) return String is
      Text : String (1 .. 10);
   begin
      for Operation in Operation_Kind loop
         Text (Operation_Kind'Pos (Operation) + 1) :=
           (if Value (Operation) then '1' else '0');
      end loop;
      return Text;
   end Operations_Text;

   procedure Write_Record (Path : String; Line : String; Success : out Boolean)
   is
      Temp : constant String := Path & ".tmp";
      FD : GNAT.OS_Lib.File_Descriptor := GNAT.OS_Lib.Invalid_FD;
      Closed : Boolean := False;
      Renamed : Boolean := False;
      Data : aliased String := Line & ASCII.LF;
      Written : Integer;
   begin
      Success := False;
      FD := GNAT.OS_Lib.Create_File (Temp, GNAT.OS_Lib.Binary);
      if FD = GNAT.OS_Lib.Invalid_FD then
         return;
      end if;
      Written := GNAT.OS_Lib.Write (FD, Data'Address, Data'Length);
      if Written /= Data'Length
        or else Fcntl (Interfaces.C.int (FD), F_FULLFSYNC) /= 0
      then
         GNAT.OS_Lib.Close (FD, Closed);
         return;
      end if;
      GNAT.OS_Lib.Close (FD, Closed);
      if not Closed then
         return;
      end if;
      GNAT.OS_Lib.Rename_File (Temp, Path, Renamed);
      if Renamed then
         Full_Sync_Directory (Ada.Directories.Containing_Directory (Path));
      end if;
      Success := Renamed;
   exception
      when others =>
         if FD /= GNAT.OS_Lib.Invalid_FD then
            GNAT.OS_Lib.Close (FD, Closed);
         end if;
         Success := False;
   end Write_Record;

   procedure Read_Record (Path : String; Line : out String; Success : out Boolean)
   is
      FD : GNAT.OS_Lib.File_Descriptor := GNAT.OS_Lib.Invalid_FD;
      Closed : Boolean := False;
      Data : aliased String (1 .. Line'Length + 2);
      Read_Count : Integer;
   begin
      Line := [others => ' '];
      Success := False;
      FD := GNAT.OS_Lib.Open_Read (Path, GNAT.OS_Lib.Binary);
      if FD = GNAT.OS_Lib.Invalid_FD then
         return;
      end if;
      Read_Count := GNAT.OS_Lib.Read (FD, Data'Address, Data'Length);
      GNAT.OS_Lib.Close (FD, Closed);
      if not Closed
        or else Read_Count /= Line'Length + 1
        or else Data (Line'Length + 1) /= ASCII.LF
      then
         return;
      end if;
      Line := Data (1 .. Line'Length);
      Success := True;
   exception
      when others =>
         if FD /= GNAT.OS_Lib.Invalid_FD then
            GNAT.OS_Lib.Close (FD, Closed);
         end if;
         Success := False;
   end Read_Record;

   function Grant_Line (Grant : Capability_Grant) return String is
     (Grant_Tag & "|" & String (Grant.Id) & "|" & String (Grant.Mission)
      & "|" & String (Grant.Subject) & "|" & Resource_Text (Grant.Resource)
      & "|" & Operations_Text (Grant.Operations) & "|" & String (Grant.Scope)
      & "|" & Sequence_Text (Grant.Expires_After) & "|"
      & Byte_Text (Grant.Maximum_Bytes) & "|"
      & (if Grant.Status = Active then "A" else "R"));

   function Approval_Line (Approval : Approval_Record) return String is
     (Approval_Tag & "|" & String (Approval.Id) & "|"
      & String (Approval.Mission) & "|" & String (Approval.Action_Digest)
      & "|" & Sequence_Text (Approval.Expires_After) & "|"
      & (case Approval.Status is
           when Approval_Active   => "A",
           when Approval_Consumed => "C",
           when Approval_Revoked  => "R"));

   procedure Parse_Grant
     (Line : String; Grant : out Capability_Grant; Success : out Boolean)
   is
      Normalized : constant String (1 .. Line'Length) := Line;
   begin
      Grant :=
        (Id            => [others => '0'],
         Mission       => [others => '0'],
         Subject       => [others => '0'],
         Resource      => Filesystem,
         Operations    => [others => False],
         Scope         => [others => '0'],
         Expires_After => 0,
         Maximum_Bytes => 0,
         Status        => Revoked);
      Success := False;
      if Normalized'Length /= Grant_Record_Length
        or else Normalized (1 .. 16) /= Grant_Tag
        or else Normalized (17) /= '|'
        or else Normalized (44) /= '|'
        or else Normalized (71) /= '|'
        or else Normalized (98) /= '|'
        or else Normalized (100) /= '|'
        or else Normalized (111) /= '|'
        or else Normalized (176) /= '|'
        or else Normalized (197) /= '|'
        or else Normalized (208) /= '|'
      then
         return;
      end if;
      if not Is_Identifier (Normalized (18 .. 43))
        or else not Is_Identifier (Normalized (45 .. 70))
        or else not Is_Identifier (Normalized (72 .. 97))
        or else Normalized (99) not in '0' .. '5'
        or else not Is_Hex (Normalized (112 .. 175))
        or else not Is_Digits (Normalized (177 .. 196))
        or else not Is_Digits (Normalized (198 .. 207))
        or else Normalized (209) not in 'A' | 'R'
      then
         return;
      end if;
      for Operation in Operation_Kind loop
         case Normalized (101 + Operation_Kind'Pos (Operation)) is
            when '0' =>
               Grant.Operations (Operation) := False;
            when '1' =>
               Grant.Operations (Operation) := True;
            when others =>
               return;
         end case;
      end loop;
      Grant.Id := Capability_Id (Normalized (18 .. 43));
      Grant.Mission := Mission_Id (Normalized (45 .. 70));
      Grant.Subject := Worker_Id (Normalized (72 .. 97));
      Grant.Resource :=
        Resource_Kind'Val (Character'Pos (Normalized (99))
                           - Character'Pos ('0'));
      Grant.Scope := Digest_256 (Normalized (112 .. 175));
      Grant.Expires_After := Sequence_Number'Value (Normalized (177 .. 196));
      Grant.Maximum_Bytes := Natural'Value (Normalized (198 .. 207));
      Grant.Status := (if Normalized (209) = 'A' then Active else Revoked);
      Success := True;
   exception
      when others =>
         Success := False;
   end Parse_Grant;

   procedure Parse_Approval
     (Line : String; Approval : out Approval_Record; Success : out Boolean)
   is
      Normalized : constant String (1 .. Line'Length) := Line;
   begin
      Approval :=
        (Id            => [others => '0'],
         Mission       => [others => '0'],
         Action_Digest => [others => '0'],
         Expires_After => 0,
         Status        => Approval_Revoked);
      Success := False;
      if Normalized'Length /= Approval_Record_Length
        or else Normalized (1 .. 19) /= Approval_Tag
        or else Normalized (20) /= '|'
        or else Normalized (47) /= '|'
        or else Normalized (74) /= '|'
        or else Normalized (139) /= '|'
        or else Normalized (160) /= '|'
      then
         return;
      end if;
      if not Is_Identifier (Normalized (21 .. 46))
        or else not Is_Identifier (Normalized (48 .. 73))
        or else not Is_Hex (Normalized (75 .. 138))
        or else not Is_Digits (Normalized (140 .. 159))
        or else Normalized (161) not in 'A' | 'C' | 'R'
      then
         return;
      end if;
      Approval.Id := Approval_Id (Normalized (21 .. 46));
      Approval.Mission := Mission_Id (Normalized (48 .. 73));
      Approval.Action_Digest := Digest_256 (Normalized (75 .. 138));
      Approval.Expires_After := Sequence_Number'Value (Normalized (140 .. 159));
      Approval.Status :=
        (case Normalized (161) is
           when 'A'    => Approval_Active,
           when 'C'    => Approval_Consumed,
           when others => Approval_Revoked);
      Success := True;
   exception
      when others =>
         Success := False;
   end Parse_Approval;

   procedure Persist_Parent_Grant
     (Home   : String;
      Grant  : Capability_Grant;
      Result : out Authority_Status)
   is
      Path : constant String := Grant_Path (Home, Grant.Mission);
      Written : Boolean;
   begin
      if not Ada.Directories.Exists (Mission_Directory (Home, Grant.Mission))
      then
         Result := Authority_IO_Failure;
         return;
      end if;
      if Ada.Directories.Exists (Path) then
         Result := Grant_Already_Exists;
         return;
      end if;
      Write_Record (Path, Grant_Line (Grant), Written);
      Result := (if Written then Authority_OK else Authority_IO_Failure);
   exception
      when others =>
         Result := Authority_IO_Failure;
   end Persist_Parent_Grant;

   procedure Load_Parent_Grant
     (Home    : String;
      Mission : Mission_Id;
      Result  : out Authority_Status;
      Grant   : out Capability_Grant)
   is
      Path : constant String := Grant_Path (Home, Mission);
      Line : String (1 .. Grant_Record_Length);
      Read_OK : Boolean;
      Parsed : Boolean;
   begin
      Grant :=
        (Id            => [others => '0'],
         Mission       => Mission,
         Subject       => [others => '0'],
         Resource      => Filesystem,
         Operations    => [others => False],
         Scope         => [others => '0'],
         Expires_After => 0,
         Maximum_Bytes => 0,
         Status        => Revoked);
      if not Ada.Directories.Exists (Path) then
         Result := Grant_Not_Found;
         return;
      end if;
      Read_Record (Path, Line, Read_OK);
      if not Read_OK then
         Result := Authority_Corrupt;
         return;
      end if;
      Parse_Grant (Line, Grant, Parsed);
      if not Parsed or else Grant.Mission /= Mission then
         Result := Authority_Corrupt;
         return;
      end if;
      Result := Authority_OK;
   exception
      when others =>
         Result := Authority_Corrupt;
   end Load_Parent_Grant;

   procedure Persist_Approval
     (Home     : String;
      Approval : Approval_Record;
      Result   : out Authority_Status)
   is
      Path : constant String :=
        Approval_Path (Home, Approval.Mission, Approval.Action_Digest);
      Written : Boolean;
   begin
      if not Ada.Directories.Exists
        (Mission_Directory (Home, Approval.Mission))
      then
         Result := Authority_IO_Failure;
         return;
      end if;
      if Ada.Directories.Exists (Path) then
         Result := Approval_Already_Exists;
         return;
      end if;
      Write_Record (Path, Approval_Line (Approval), Written);
      Result := (if Written then Authority_OK else Authority_IO_Failure);
   exception
      when others =>
         Result := Authority_IO_Failure;
   end Persist_Approval;

   procedure Load_Approval
     (Home          : String;
      Mission       : Mission_Id;
      Action_Digest : Digest_256;
      Result        : out Authority_Status;
      Approval      : out Approval_Record)
   is
      Path : constant String := Approval_Path (Home, Mission, Action_Digest);
      Line : String (1 .. Approval_Record_Length);
      Read_OK : Boolean;
      Parsed : Boolean;
   begin
      Approval :=
        (Id            => [others => '0'],
         Mission       => Mission,
         Action_Digest => Action_Digest,
         Expires_After => 0,
         Status        => Approval_Revoked);
      if not Ada.Directories.Exists (Path) then
         Result := Approval_Not_Found;
         return;
      end if;
      Read_Record (Path, Line, Read_OK);
      if not Read_OK then
         Result := Authority_Corrupt;
         return;
      end if;
      Parse_Approval (Line, Approval, Parsed);
      if not Parsed
        or else Approval.Mission /= Mission
        or else Approval.Action_Digest /= Action_Digest
      then
         Result := Authority_Corrupt;
         return;
      end if;
      Result := Authority_OK;
   exception
      when others =>
         Result := Authority_Corrupt;
   end Load_Approval;

   procedure Persist_Consumed_Approval
     (Home     : String;
      Approval : Approval_Record;
      Result   : out Authority_Status)
   is
      Path : constant String :=
        Approval_Path (Home, Approval.Mission, Approval.Action_Digest);
      Written : Boolean;
   begin
      if Approval.Status /= Approval_Consumed then
         Result := Authority_IO_Failure;
         return;
      end if;
      if not Ada.Directories.Exists (Path) then
         Result := Approval_Not_Found;
         return;
      end if;
      Write_Record (Path, Approval_Line (Approval), Written);
      Result := (if Written then Authority_OK else Authority_IO_Failure);
   exception
      when others =>
         Result := Authority_IO_Failure;
   end Persist_Consumed_Approval;
end Nemesis.Core.Authority_Store;
