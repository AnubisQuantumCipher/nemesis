with Ada.Directories;
with Ada.Strings;
with Ada.Strings.Fixed;
with Interfaces.C;
with Interfaces.C.Strings;
with System;

package body Nemesis.Core.Index with SPARK_Mode => Off is
   use type Interfaces.C.int;
   use type Interfaces.C.long_long;
   use type Interfaces.C.Strings.chars_ptr;
   use type System.Address;

   SQLite_OK   : constant Interfaces.C.int := 0;
   SQLite_ROW  : constant Interfaces.C.int := 100;
   SQLite_DONE : constant Interfaces.C.int := 101;
   SQLite_Open_Read_Write : constant Interfaces.C.int := 2;
   SQLite_Open_Create     : constant Interfaces.C.int := 4;
   SQLite_Open_Full_Mutex : constant Interfaces.C.int := 65_536;

   function SQLite_Open_V2
     (Filename : Interfaces.C.Strings.chars_ptr;
      Database : access System.Address;
      Flags    : Interfaces.C.int;
      VFS      : Interfaces.C.Strings.chars_ptr) return Interfaces.C.int
   with Import, Convention => C, External_Name => "sqlite3_open_v2";

   function SQLite_Close_V2
     (Database : System.Address) return Interfaces.C.int
   with Import, Convention => C, External_Name => "sqlite3_close_v2";

   function SQLite_Exec
     (Database      : System.Address;
      SQL           : Interfaces.C.Strings.chars_ptr;
      Callback      : System.Address;
      Callback_Data : System.Address;
      Error_Message : System.Address) return Interfaces.C.int
   with Import, Convention => C, External_Name => "sqlite3_exec";

   function SQLite_Prepare_V2
     (Database  : System.Address;
      SQL       : Interfaces.C.Strings.chars_ptr;
      Byte_Count : Interfaces.C.int;
      Statement : access System.Address;
      Tail      : System.Address) return Interfaces.C.int
   with Import, Convention => C, External_Name => "sqlite3_prepare_v2";

   function SQLite_Step (Statement : System.Address) return Interfaces.C.int
   with Import, Convention => C, External_Name => "sqlite3_step";

   function SQLite_Finalize
     (Statement : System.Address) return Interfaces.C.int
   with Import, Convention => C, External_Name => "sqlite3_finalize";

   function SQLite_Column_Int64
     (Statement : System.Address;
      Column    : Interfaces.C.int) return Interfaces.C.long_long
   with Import, Convention => C, External_Name => "sqlite3_column_int64";

   function SQLite_Column_Text
     (Statement : System.Address;
      Column    : Interfaces.C.int) return Interfaces.C.Strings.chars_ptr
   with Import, Convention => C, External_Name => "sqlite3_column_text";

   function Is_Hex (Value : Digest_Hex) return Boolean is
   begin
      for Item of Value loop
         if not (Item in '0' .. '9' | 'a' .. 'f') then
            return False;
         end if;
      end loop;
      return True;
   end Is_Hex;

   function Is_Valid_Id (Id : Mission_Id) return Boolean is
   begin
      if Id (Id'First .. Id'First + 3) /= "mis_" then
         return False;
      end if;
      for Index in Id'First + 4 .. Id'Last loop
         if not (Id (Index) in '0' .. '9' | 'a' .. 'z') then
            return False;
         end if;
      end loop;
      return True;
   end Is_Valid_Id;

   function Image (Value : Long_Long_Integer) return String is
     (Ada.Strings.Fixed.Trim
        (Long_Long_Integer'Image (Value), Ada.Strings.Both));

   procedure Open_Database
     (Path    : String;
      DB      : aliased out System.Address;
      Success : out Boolean)
   is
      Name : Interfaces.C.Strings.chars_ptr :=
        Interfaces.C.Strings.New_String (Path);
      Code : Interfaces.C.int;
   begin
      DB := System.Null_Address;
      Code :=
        SQLite_Open_V2
          (Name,
           DB'Access,
           SQLite_Open_Read_Write
             + SQLite_Open_Create
             + SQLite_Open_Full_Mutex,
           Interfaces.C.Strings.Null_Ptr);
      Interfaces.C.Strings.Free (Name);
      Success := Code = SQLite_OK and then DB /= System.Null_Address;
      if not Success and then DB /= System.Null_Address then
         Code := SQLite_Close_V2 (DB);
         DB := System.Null_Address;
      end if;
   exception
      when others =>
         if Name /= Interfaces.C.Strings.Null_Ptr then
            Interfaces.C.Strings.Free (Name);
         end if;
         if DB /= System.Null_Address then
            Code := SQLite_Close_V2 (DB);
            DB := System.Null_Address;
         end if;
         Success := False;
   end Open_Database;

   function Execute (DB : System.Address; SQL_Text : String) return Boolean is
      SQL : Interfaces.C.Strings.chars_ptr :=
        Interfaces.C.Strings.New_String (SQL_Text);
      Code : Interfaces.C.int;
   begin
      Code :=
        SQLite_Exec
          (DB, SQL, System.Null_Address, System.Null_Address,
           System.Null_Address);
      Interfaces.C.Strings.Free (SQL);
      return Code = SQLite_OK;
   exception
      when others =>
         if SQL /= Interfaces.C.Strings.Null_Ptr then
            Interfaces.C.Strings.Free (SQL);
         end if;
         return False;
   end Execute;

   procedure Prepare
     (DB       : System.Address;
      SQL_Text : String;
      Statement : aliased out System.Address;
      Success  : out Boolean)
   is
      SQL : Interfaces.C.Strings.chars_ptr :=
        Interfaces.C.Strings.New_String (SQL_Text);
      Code : Interfaces.C.int;
   begin
      Statement := System.Null_Address;
      Code :=
        SQLite_Prepare_V2
          (DB, SQL, -1, Statement'Access, System.Null_Address);
      Interfaces.C.Strings.Free (SQL);
      Success := Code = SQLite_OK and then Statement /= System.Null_Address;
   exception
      when others =>
         if SQL /= Interfaces.C.Strings.Null_Ptr then
            Interfaces.C.Strings.Free (SQL);
         end if;
         Statement := System.Null_Address;
         Success := False;
   end Prepare;

   function Read_User_Version
     (DB : System.Address; Version : out Natural) return Boolean
   is
      Statement : aliased System.Address := System.Null_Address;
      Prepared  : Boolean;
      Code      : Interfaces.C.int;
      Finalized : Interfaces.C.int;
      Raw       : Interfaces.C.long_long;
   begin
      Version := 0;
      Prepare (DB, "PRAGMA user_version", Statement, Prepared);
      if not Prepared then
         return False;
      end if;
      Code := SQLite_Step (Statement);
      if Code /= SQLite_ROW then
         Finalized := SQLite_Finalize (Statement);
         return False;
      end if;
      Raw := SQLite_Column_Int64 (Statement, 0);
      Finalized := SQLite_Finalize (Statement);
      if Finalized /= SQLite_OK
        or else Raw < 0
        or else Raw > Interfaces.C.long_long (Natural'Last)
      then
         return False;
      end if;
      Version := Natural (Raw);
      return True;
   end Read_User_Version;

   procedure Initialize
     (Path    : String;
      Result  : out Index_Result;
      Version : out Natural)
   is
      DB      : aliased System.Address := System.Null_Address;
      Opened  : Boolean;
      Closed  : Interfaces.C.int;
      Directory : constant String := Ada.Directories.Containing_Directory (Path);
   begin
      Result := Index_Database_Error;
      Version := 0;
      if not Ada.Directories.Exists (Directory) then
         Ada.Directories.Create_Path (Directory);
      end if;
      Open_Database (Path, DB, Opened);
      if not Opened then
         return;
      end if;

      if not Execute (DB, "PRAGMA journal_mode=WAL")
        or else not Execute (DB, "PRAGMA synchronous=FULL")
        or else not Execute (DB, "PRAGMA foreign_keys=ON")
        or else not Read_User_Version (DB, Version)
      then
         Closed := SQLite_Close_V2 (DB);
         return;
      end if;

      if Version = 0 then
         if not Execute (DB, "BEGIN IMMEDIATE")
           or else not Execute
             (DB,
              "CREATE TABLE IF NOT EXISTS missions ("
              & "mission_id TEXT PRIMARY KEY NOT NULL CHECK(length(mission_id)=26),"
              & "sequence INTEGER NOT NULL CHECK(sequence>=0),"
              & "state INTEGER NOT NULL CHECK(state>=0 AND state<=10),"
              & "ledger_head TEXT NOT NULL CHECK(length(ledger_head)=64),"
              & "source_digest TEXT NOT NULL CHECK(length(source_digest)=64))")
           or else not Execute
             (DB,
              "CREATE TABLE IF NOT EXISTS events ("
              & "mission_id TEXT NOT NULL,"
              & "sequence INTEGER NOT NULL CHECK(sequence>0),"
              & "event_digest TEXT NOT NULL CHECK(length(event_digest)=64),"
              & "PRIMARY KEY(mission_id, sequence),"
              & "FOREIGN KEY(mission_id) REFERENCES missions(mission_id))")
           or else not Execute (DB, "PRAGMA user_version=1")
           or else not Execute (DB, "COMMIT")
         then
            if not Execute (DB, "ROLLBACK") then
               null;
            end if;
            Closed := SQLite_Close_V2 (DB);
            return;
         end if;
         Version := Supported_Schema_Version;
      elsif Version > Supported_Schema_Version then
         Result := Index_Unsupported_Schema;
         Closed := SQLite_Close_V2 (DB);
         return;
      end if;

      Closed := SQLite_Close_V2 (DB);
      if Closed = SQLite_OK then
         Result := Index_OK;
      end if;
   exception
      when others =>
         if DB /= System.Null_Address then
            Closed := SQLite_Close_V2 (DB);
         end if;
         Result := Index_Database_Error;
   end Initialize;

   procedure Upsert_Mission
     (Path   : String;
      Id     : Mission_Id;
      Value  : Indexed_Mission;
      Result : out Index_Result)
   is
      Version : Natural;
      DB      : aliased System.Address := System.Null_Address;
      Opened  : Boolean;
      Closed  : Interfaces.C.int;
      Updated : Boolean;
   begin
      if not Is_Valid_Id (Id)
        or else not Is_Hex (Value.Ledger_Head)
        or else not Is_Hex (Value.Source_Digest)
      then
         Result := Index_Invalid_Input;
         return;
      end if;
      Initialize (Path, Result, Version);
      if Result /= Index_OK or else Version /= Supported_Schema_Version then
         return;
      end if;
      Open_Database (Path, DB, Opened);
      if not Opened then
         Result := Index_Database_Error;
         return;
      end if;

      Updated :=
        Execute
          (DB,
           "INSERT INTO missions"
           & "(mission_id,sequence,state,ledger_head,source_digest) VALUES ('"
           & String (Id) & "',"
           & Image (Long_Long_Integer (Value.Sequence)) & ","
           & Image (Long_Long_Integer (Mission_State'Pos (Value.State))) & ",'"
           & Value.Ledger_Head & "','" & Value.Source_Digest & "') "
           & "ON CONFLICT(mission_id) DO UPDATE SET "
           & "sequence=excluded.sequence,state=excluded.state,"
           & "ledger_head=excluded.ledger_head,"
           & "source_digest=excluded.source_digest");
      Closed := SQLite_Close_V2 (DB);
      if Updated and then Closed = SQLite_OK then
         Result := Index_OK;
      else
         Result := Index_Database_Error;
      end if;
   exception
      when others =>
         if DB /= System.Null_Address then
            Closed := SQLite_Close_V2 (DB);
         end if;
         Result := Index_Database_Error;
   end Upsert_Mission;

   procedure Read_Mission
     (Path   : String;
      Id     : Mission_Id;
      Result : out Index_Result;
      Found  : out Boolean;
      Value  : out Indexed_Mission)
   is
      Version  : Natural;
      DB       : aliased System.Address := System.Null_Address;
      Statement : aliased System.Address := System.Null_Address;
      Opened   : Boolean;
      Prepared : Boolean;
      Code     : Interfaces.C.int;
      Finalized : Interfaces.C.int;
      Closed   : Interfaces.C.int;
      Ledger_Text : Interfaces.C.Strings.chars_ptr;
      Source_Text : Interfaces.C.Strings.chars_ptr;
   begin
      Found := False;
      Value :=
        (Sequence      => Sequence_Number'First,
         State         => Draft,
         Ledger_Head   => Zero_Digest,
         Source_Digest => Zero_Digest);
      if not Is_Valid_Id (Id) then
         Result := Index_Invalid_Input;
         return;
      end if;
      Initialize (Path, Result, Version);
      if Result /= Index_OK or else Version /= Supported_Schema_Version then
         return;
      end if;
      Open_Database (Path, DB, Opened);
      if not Opened then
         Result := Index_Database_Error;
         return;
      end if;
      Prepare
        (DB,
         "SELECT sequence,state,ledger_head,source_digest FROM missions "
         & "WHERE mission_id='" & String (Id) & "'",
         Statement,
         Prepared);
      if not Prepared then
         Closed := SQLite_Close_V2 (DB);
         Result := Index_Database_Error;
         return;
      end if;

      Code := SQLite_Step (Statement);
      if Code = SQLite_ROW then
         Value.Sequence :=
           Sequence_Number (SQLite_Column_Int64 (Statement, 0));
         Value.State :=
           Decode_State
             (State_Code
                (Integer (SQLite_Column_Int64 (Statement, 1))));
         Ledger_Text := SQLite_Column_Text (Statement, 2);
         Source_Text := SQLite_Column_Text (Statement, 3);
         if Ledger_Text = Interfaces.C.Strings.Null_Ptr
           or else Source_Text = Interfaces.C.Strings.Null_Ptr
         then
            Finalized := SQLite_Finalize (Statement);
            Closed := SQLite_Close_V2 (DB);
            Result := Index_Database_Error;
            return;
         end if;
         declare
            Ledger_Value : constant String :=
              Interfaces.C.Strings.Value (Ledger_Text);
            Source_Value : constant String :=
              Interfaces.C.Strings.Value (Source_Text);
         begin
            if Ledger_Value'Length /= Digest_Hex'Length
              or else Source_Value'Length /= Digest_Hex'Length
            then
               Finalized := SQLite_Finalize (Statement);
               Closed := SQLite_Close_V2 (DB);
               Result := Index_Database_Error;
               return;
            end if;
            Value.Ledger_Head := Digest_Hex (Ledger_Value);
            Value.Source_Digest := Digest_Hex (Source_Value);
         end;
         if not Is_Hex (Value.Ledger_Head)
           or else not Is_Hex (Value.Source_Digest)
         then
            Finalized := SQLite_Finalize (Statement);
            Closed := SQLite_Close_V2 (DB);
            Result := Index_Database_Error;
            return;
         end if;
         Found := True;
      elsif Code /= SQLite_DONE then
         Finalized := SQLite_Finalize (Statement);
         Closed := SQLite_Close_V2 (DB);
         Result := Index_Database_Error;
         return;
      end if;

      Finalized := SQLite_Finalize (Statement);
      Closed := SQLite_Close_V2 (DB);
      if Finalized = SQLite_OK and then Closed = SQLite_OK then
         Result := Index_OK;
      else
         Found := False;
         Result := Index_Database_Error;
      end if;
   exception
      when others =>
         if Statement /= System.Null_Address then
            Finalized := SQLite_Finalize (Statement);
         end if;
         if DB /= System.Null_Address then
            Closed := SQLite_Close_V2 (DB);
         end if;
         Found := False;
         Result := Index_Database_Error;
   end Read_Mission;

end Nemesis.Core.Index;
