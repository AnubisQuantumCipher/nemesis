with Ada.Directories;
with Ada.Strings.Unbounded;
with GNAT.OS_Lib;
with Interfaces.C;
with Nemesis.Core.Checkpoints;
with Nemesis.Core.Objects;
with Nemesis.Kernel.Transitions;

package body Nemesis.Core.Mission_Store with SPARK_Mode => Off is
   use Ada.Strings.Unbounded;
   use Nemesis.Core.Checkpoints;
   use Nemesis.Core.Objects;
   use Nemesis.Kernel.Transitions;
   use type GNAT.OS_Lib.File_Descriptor;
   use type Interfaces.C.int;

   Zero : constant Digest_256 := [others => '0'];
   Contract_Tag : constant String := "NEMESIS_CONTRACT_V1";
   Evidence_Tag : constant String := "NEMESIS_EVIDENCE_V1";

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

   function Ledger_Path (Home : String; Id : Mission_Id) return String is
     (Mission_Directory (Home, Id) & "/events.ledger");

   function Checkpoint_Path (Home : String; Id : Mission_Id) return String is
     (Mission_Directory (Home, Id) & "/checkpoint.ncp");

   function Contract_Reference_Path
     (Home : String; Id : Mission_Id) return String
   is
     (Mission_Directory (Home, Id) & "/contract.ref");

   function Evidence_Reference_Path
     (Home : String; Id : Mission_Id; Claim : String) return String
   is
     (Mission_Directory (Home, Id) & "/evidence-" & Claim & ".ref");

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

   function Empty_Context (Id : Mission_Id) return Mission_Context is
     ((Mission          => Create,
       Id               => Id,
       Worker           => [others => '0'],
       Contract_Digest  => Zero,
       Scope_Digest     => Zero,
       Current_Source   => Zero,
       Ledger_Head      => Zero,
       Build_Source     => Zero,
       Build_Evidence   => Zero,
       Test_Source      => Zero,
       Test_Evidence    => Zero));

   procedure Write_Reference
     (Path : String; Value : Digest_256; Success : out Boolean)
   is
      Temp : constant String := Path & ".tmp";
      FD : GNAT.OS_Lib.File_Descriptor := GNAT.OS_Lib.Invalid_FD;
      Closed : Boolean := False;
      Renamed : Boolean := False;
      Data : aliased String := String (Value) & ASCII.LF;
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
   end Write_Reference;

   procedure Read_Reference
     (Path : String; Value : out Digest_256; Success : out Boolean)
   is
      FD : GNAT.OS_Lib.File_Descriptor := GNAT.OS_Lib.Invalid_FD;
      Closed : Boolean := False;
      Data : aliased String (1 .. 65);
      Read_Count : Integer;
   begin
      Value := Zero;
      Success := False;
      FD := GNAT.OS_Lib.Open_Read (Path, GNAT.OS_Lib.Binary);
      if FD = GNAT.OS_Lib.Invalid_FD then
         return;
      end if;
      Read_Count := GNAT.OS_Lib.Read (FD, Data'Address, Data'Length);
      GNAT.OS_Lib.Close (FD, Closed);
      if not Closed
        or else Read_Count /= Data'Length
        or else Data (Data'Last) /= ASCII.LF
        or else not Is_Hex (Data (1 .. 64))
      then
         return;
      end if;
      Value := Digest_256 (Data (1 .. 64));
      Success := True;
   exception
      when others =>
         if FD /= GNAT.OS_Lib.Invalid_FD then
            GNAT.OS_Lib.Close (FD, Closed);
         end if;
         Value := Zero;
         Success := False;
   end Read_Reference;

   function Contract_Object
     (Id              : Mission_Id;
      Worker          : Worker_Id;
      Contract_Digest : Digest_256;
      Scope_Digest    : Digest_256;
      Initial_Source  : Digest_256) return String
   is
     (Contract_Tag & "|" & String (Id) & "|" & String (Worker) & "|"
      & String (Contract_Digest) & "|" & String (Scope_Digest) & "|"
      & String (Initial_Source));

   procedure Parse_Contract
     (Data    : String;
      Context : in out Mission_Context;
      Success : out Boolean)
   is
      Normalized : constant String (1 .. Data'Length) := Data;
   begin
      Success := False;
      if Normalized'Length /= 268
        or else Normalized (1 .. 19) /= Contract_Tag
        or else Normalized (20) /= '|'
        or else Normalized (47) /= '|'
        or else Normalized (74) /= '|'
        or else Normalized (139) /= '|'
        or else Normalized (204) /= '|'
      then
         return;
      end if;
      if Normalized (21 .. 46) /= String (Context.Id)
        or else not Is_Hex (Normalized (75 .. 138))
        or else not Is_Hex (Normalized (140 .. 203))
        or else not Is_Hex (Normalized (205 .. 268))
      then
         return;
      end if;
      Context.Worker := Worker_Id (Normalized (48 .. 73));
      Context.Contract_Digest := Digest_256 (Normalized (75 .. 138));
      Context.Scope_Digest := Digest_256 (Normalized (140 .. 203));
      Context.Current_Source := Digest_256 (Normalized (205 .. 268));
      Success := True;
   end Parse_Contract;

   procedure Save_Checkpoint
     (Home : String; Context : Mission_Context; Success : out Boolean)
   is
      Status : Checkpoint_Write_Result;
      Checkpoint_Digest : Nemesis.Core.Ledger.Digest_Hex;
   begin
      Write_Checkpoint
        (Path => Checkpoint_Path (Home, Context.Id),
         Value =>
           (Sequence      => Sequence_Of (Context.Mission),
            State         => State_Of (Context.Mission),
            Ledger_Head   => Nemesis.Core.Ledger.Digest_Hex (Context.Ledger_Head),
            Source_Digest => Nemesis.Core.Ledger.Digest_Hex (Context.Current_Source)),
         Result => Status,
         Checkpoint_Digest => Checkpoint_Digest);
      Success := Status = Checkpoint_Committed;
   end Save_Checkpoint;

   procedure Append_Current_Event
     (Home    : String;
      Context : in out Mission_Context;
      Kind    : Event_Kind;
      Payload : Digest_256;
      Result  : out Store_Status)
   is
      Status : Append_Result;
      Event_Hash : Nemesis.Core.Ledger.Digest_Hex;
      Recovered : Recovery_Result;
      Checkpoint_OK : Boolean;
   begin
      Append
        (Path => Ledger_Path (Home, Context.Id),
         Kind => Kind,
         State => State_Of (Context.Mission),
         Source_Digest =>
           Nemesis.Core.Ledger.Digest_Hex (Context.Current_Source),
         Payload_Digest => Nemesis.Core.Ledger.Digest_Hex (Payload),
         Result => Status,
         Event_Digest => Event_Hash);
      if Status /= Committed then
         Result := Store_IO_Failure;
         return;
      end if;
      Recover (Ledger_Path (Home, Context.Id), Recovered);
      if Recovered.Status /= Recovered_Valid
        or else Recovered.Sequence /= Sequence_Of (Context.Mission)
        or else Recovered.State /= State_Of (Context.Mission)
        or else Recovered.Head /= Event_Hash
      then
         Result := Store_Corrupt;
         return;
      end if;
      Context.Ledger_Head := Digest_256 (Event_Hash);
      Save_Checkpoint (Home, Context, Checkpoint_OK);
      if Checkpoint_OK then
         Result := Store_OK;
      else
         Result := Store_IO_Failure;
      end if;
   end Append_Current_Event;

   procedure Commit_Transition
     (Home       : String;
      Context    : in out Mission_Context;
      Target     : Mission_State;
      Kind       : Event_Kind;
      Payload    : Digest_256;
      Result     : out Store_Status)
   is
      Before : constant Mission_Record := Context.Mission;
      Decision : Transition_Decision;
   begin
      Apply (Context.Mission, Target, Decision);
      if Decision /= Accepted then
         Result := Store_Refused;
         return;
      end if;
      Append_Current_Event (Home, Context, Kind, Payload, Result);
      if Result /= Store_OK then
         Context.Mission := Before;
      end if;
   end Commit_Transition;

   procedure Commit_Event
     (Home       : String;
      Context    : in out Mission_Context;
      Kind       : Event_Kind;
      Payload    : Digest_256;
      Result     : out Store_Status)
   is
      Before : constant Mission_Record := Context.Mission;
      Decision : Transition_Decision;
   begin
      Nemesis.Kernel.Missions.Commit_Event (Context.Mission, Decision);
      if Decision /= Accepted then
         Result := Store_Refused;
         return;
      end if;
      Append_Current_Event (Home, Context, Kind, Payload, Result);
      if Result /= Store_OK then
         Context.Mission := Before;
      end if;
   end Commit_Event;

   procedure Create_Mission
     (Home             : String;
      Id               : Mission_Id;
      Worker           : Worker_Id;
      Contract_Digest  : Digest_256;
      Scope_Digest     : Digest_256;
      Initial_Source   : Digest_256;
      Result           : out Store_Status;
      Context          : out Mission_Context)
   is
      Directory : constant String := Mission_Directory (Home, Id);
      Data : constant String :=
        Contract_Object
          (Id, Worker, Contract_Digest, Scope_Digest, Initial_Source);
      Object_Status : Store_Result;
      Object_Id : Nemesis.Core.Ledger.Digest_Hex;
      Reference_OK : Boolean;
   begin
      Context := Empty_Context (Id);
      if Ada.Directories.Exists (Directory) then
         Result := Mission_Already_Exists;
         return;
      end if;
      if not Is_Hex (String (Contract_Digest))
        or else not Is_Hex (String (Scope_Digest))
        or else not Is_Hex (String (Initial_Source))
      then
         Result := Store_Refused;
         return;
      end if;
      Ada.Directories.Create_Path (Directory);
      Put (Home, Data, Object_Status, Object_Id);
      if Object_Status not in Stored | Already_Present then
         Result := Store_IO_Failure;
         return;
      end if;
      Write_Reference
        (Contract_Reference_Path (Home, Id), Digest_256 (Object_Id), Reference_OK);
      if not Reference_OK then
         Result := Store_IO_Failure;
         return;
      end if;

      Context.Worker := Worker;
      Context.Contract_Digest := Contract_Digest;
      Context.Scope_Digest := Scope_Digest;
      Context.Current_Source := Initial_Source;
      Commit_Transition
        (Home, Context, Contract_Compiled, Mission_Created,
         Digest_256 (Object_Id), Result);
      if Result /= Store_OK then
         return;
      end if;
      Commit_Transition
        (Home, Context, Awaiting_Authorization, Contract_Compiled_Event,
         Digest_256 (Object_Id), Result);
   exception
      when others =>
         Context := Empty_Context (Id);
         Result := Store_IO_Failure;
   end Create_Mission;

   procedure Load_Evidence
     (Home    : String;
      Context : in out Mission_Context;
      Claim   : String;
      Success : out Boolean)
   is
      Reference : Digest_256;
      Reference_OK : Boolean;
      Load_Status : Load_Result;
      Data : Unbounded_String;
      Text : Unbounded_String;
   begin
      Success := True;
      if not Ada.Directories.Exists
        (Evidence_Reference_Path (Home, Context.Id, Claim))
      then
         return;
      end if;
      Read_Reference
        (Evidence_Reference_Path (Home, Context.Id, Claim), Reference,
         Reference_OK);
      if not Reference_OK then
         Success := False;
         return;
      end if;
      Get (Home, Nemesis.Core.Ledger.Digest_Hex (Reference), Load_Status, Data);
      if Load_Status /= Loaded_Valid then
         Success := False;
         return;
      end if;
      Text := Data;
      if Length (Text) /= 155 then
         Success := False;
         return;
      end if;
      declare
         Value : constant String := To_String (Text);
      begin
         if Value (1 .. 19) /= Evidence_Tag
           or else Value (20) /= '|'
           or else Value (26) /= '|'
           or else Value (91) /= '|'
           or else Value (21 .. 25) /= Claim
           or else not Is_Hex (Value (27 .. 90))
           or else not Is_Hex (Value (92 .. 155))
         then
            Success := False;
            return;
         end if;
         if Claim = "build" then
            Context.Build_Source := Digest_256 (Value (27 .. 90));
            Context.Build_Evidence := Digest_256 (Value (92 .. 155));
         else
            Context.Test_Source := Digest_256 (Value (27 .. 90));
            Context.Test_Evidence := Digest_256 (Value (92 .. 155));
         end if;
      end;
   end Load_Evidence;

   procedure Load_Mission
     (Home    : String;
      Id      : Mission_Id;
      Result  : out Store_Status;
      Context : out Mission_Context)
   is
      Reference : Digest_256;
      Reference_OK : Boolean;
      Object_Status : Load_Result;
      Object_Data : Unbounded_String;
      Contract_OK : Boolean;
      Recovered : Recovery_Result;
      Checkpoint_Status : Checkpoint_Read_Result;
      Checkpoint : Checkpoint_Record;
      Evidence_OK : Boolean;
   begin
      Context := Empty_Context (Id);
      if not Ada.Directories.Exists (Mission_Directory (Home, Id)) then
         Result := Mission_Not_Found;
         return;
      end if;
      Read_Reference (Contract_Reference_Path (Home, Id), Reference, Reference_OK);
      if not Reference_OK then
         Result := Store_Corrupt;
         return;
      end if;
      Get
        (Home, Nemesis.Core.Ledger.Digest_Hex (Reference), Object_Status,
         Object_Data);
      if Object_Status /= Loaded_Valid then
         Result := Store_Corrupt;
         return;
      end if;
      Parse_Contract (To_String (Object_Data), Context, Contract_OK);
      if not Contract_OK then
         Result := Store_Corrupt;
         return;
      end if;
      Recover (Ledger_Path (Home, Id), Recovered);
      Read_Checkpoint
        (Checkpoint_Path (Home, Id), Checkpoint_Status, Checkpoint);
      if Recovered.Status /= Recovered_Valid
        or else Recovered.First_Payload /=
          Nemesis.Core.Ledger.Digest_Hex (Reference)
      then
         --  The append-only, hash-chain-validated ledger is the sole source of
         --  truth; without a valid ledger on the expected chain there is nothing
         --  authoritative to load.
         Result := Store_Corrupt;
         return;
      end if;
      --  A valid checkpoint may never be ahead of the authoritative ledger: the
      --  ledger record is appended and fsynced before the checkpoint is written.
      if Checkpoint_Status = Checkpoint_Valid
        and then Checkpoint.Sequence > Recovered.Sequence
      then
         Result := Store_Corrupt;
         return;
      end if;
      Context.Mission := Restore (Recovered.State, Recovered.Sequence);
      Context.Current_Source := Digest_256 (Recovered.Source);
      Context.Ledger_Head := Digest_256 (Recovered.Head);
      --  Rebuild the derived checkpoint from the ledger tail whenever it is
      --  missing or stale, so a crash between the durable ledger append and the
      --  checkpoint rename cannot brick an otherwise intact mission (SQL-001).
      if Checkpoint_Status /= Checkpoint_Valid
        or else Checkpoint.Sequence /= Recovered.Sequence
        or else Checkpoint.State /= Recovered.State
        or else Checkpoint.Ledger_Head /= Recovered.Head
        or else Checkpoint.Source_Digest /= Recovered.Source
      then
         declare
            Rebuilt_OK : Boolean;
         begin
            Save_Checkpoint (Home, Context, Rebuilt_OK);
            if not Rebuilt_OK then
               Result := Store_IO_Failure;
               return;
            end if;
         end;
      end if;
      Load_Evidence (Home, Context, "build", Evidence_OK);
      if not Evidence_OK then
         Result := Store_Corrupt;
         return;
      end if;
      Load_Evidence (Home, Context, "tests", Evidence_OK);
      if not Evidence_OK then
         Result := Store_Corrupt;
         return;
      end if;
      Result := Store_OK;
   exception
      when others =>
         Context := Empty_Context (Id);
         Result := Store_Corrupt;
   end Load_Mission;

   procedure Record_Source
     (Home       : String;
      Context    : in out Mission_Context;
      New_Source : Digest_256;
      Action     : Digest_256;
      Result     : out Store_Status)
   is
      Before_Mission : constant Mission_Record := Context.Mission;
      Before_Source  : constant Digest_256 := Context.Current_Source;
      Decision : Transition_Decision;
      Payload : constant Digest_256 :=
        Digest_256
          (Nemesis.Core.Ledger.Digest
             ("NEMESIS_ACTION_RESULT_V1|" & String (Action) & "|"
              & String (New_Source)));
   begin
      if not Is_Hex (String (New_Source)) or else not Is_Hex (String (Action))
      then
         Result := Store_Refused;
         return;
      end if;
      Nemesis.Kernel.Missions.Commit_Event (Context.Mission, Decision);
      if Decision /= Accepted then
         Result := Store_Refused;
         return;
      end if;
      Context.Current_Source := New_Source;
      Append_Current_Event
        (Home, Context, Action_Completed, Payload, Result);
      if Result /= Store_OK then
         Context.Mission := Before_Mission;
         Context.Current_Source := Before_Source;
      end if;
   end Record_Source;

   procedure Record_Evidence
     (Home      : String;
      Context   : in out Mission_Context;
      Claim     : String;
      Source    : Digest_256;
      Evidence  : Digest_256;
      Result    : out Store_Status)
   is
      Data : constant String :=
        Evidence_Tag & "|" & Claim & "|" & String (Source) & "|"
        & String (Evidence);
      Object_Status : Store_Result;
      Object_Id : Nemesis.Core.Ledger.Digest_Hex;
      Reference_OK : Boolean;
   begin
      if Claim not in "build" | "tests"
        or else Source /= Context.Current_Source
        or else not Is_Hex (String (Evidence))
      then
         if Source /= Context.Current_Source then
            Result := Evidence_Stale;
         else
            Result := Store_Refused;
         end if;
         return;
      end if;
      Put (Home, Data, Object_Status, Object_Id);
      if Object_Status not in Stored | Already_Present then
         Result := Store_IO_Failure;
         return;
      end if;
      Commit_Event
        (Home, Context, Evidence_Accepted, Digest_256 (Object_Id), Result);
      if Result /= Store_OK then
         return;
      end if;
      Write_Reference
        (Evidence_Reference_Path (Home, Context.Id, Claim),
         Digest_256 (Object_Id), Reference_OK);
      if not Reference_OK then
         Result := Store_IO_Failure;
         return;
      end if;
      if Claim = "build" then
         Context.Build_Source := Source;
         Context.Build_Evidence := Evidence;
      else
         Context.Test_Source := Source;
         Context.Test_Evidence := Evidence;
      end if;
      Result := Store_OK;
   exception
      when others =>
         Result := Store_IO_Failure;
   end Record_Evidence;

   function Claims_Current (Context : Mission_Context) return Boolean is
     (Context.Build_Evidence /= Zero
      and then Context.Test_Evidence /= Zero
      and then Context.Build_Source = Context.Current_Source
      and then Context.Test_Source = Context.Current_Source);

end Nemesis.Core.Mission_Store;
