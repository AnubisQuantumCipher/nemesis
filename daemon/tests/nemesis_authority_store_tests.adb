with Ada.Directories;
with Ada.Text_IO;
with Nemesis.Core.Authority_Store;
with Nemesis.Kernel.Approvals;
with Nemesis.Kernel.Capabilities;
with Nemesis.Kernel.Types;

procedure Nemesis_Authority_Store_Tests is
   use Ada.Directories;
   use Ada.Text_IO;
   use Nemesis.Core.Authority_Store;
   use Nemesis.Kernel.Approvals;
   use Nemesis.Kernel.Capabilities;
   use Nemesis.Kernel.Types;

   Test_Root : constant String := "build/test-authority-store";
   Mission_A : constant Mission_Id := "mis_0000000000000000000000";
   Mission_B : constant Mission_Id := "mis_1111111111111111111111";
   Worker_A  : constant Worker_Id := "wrk_0000000000000000000000";
   Cap_A     : constant Capability_Id := "cap_0000000000000000000000";
   Apr_A     : constant Approval_Id := "apr_0000000000000000000000";
   Scope_A   : constant Digest_256 := [others => 'a'];
   Action_A  : constant Digest_256 := [others => 'c'];
   Action_B  : constant Digest_256 := [others => 'd'];

   Parent : constant Capability_Grant :=
     (Id            => Cap_A,
      Mission       => Mission_A,
      Subject       => Worker_A,
      Resource      => Filesystem,
      Operations    => [Modify_Data => True, others => False],
      Scope         => Scope_A,
      Expires_After => Sequence_Number'Last,
      Maximum_Bytes => 4_096,
      Status        => Active);

   Approval : constant Approval_Record :=
     (Id            => Apr_A,
      Mission       => Mission_A,
      Action_Digest => Action_A,
      Expires_After => 10_000,
      Status        => Approval_Active);

   Result : Authority_Status;
   Loaded_Grant : Capability_Grant;
   Loaded_Approval : Approval_Record;

   Grant_File : constant String :=
     Test_Root & "/missions/" & String (Mission_A) & "/parent.grant";
   Approval_File : constant String :=
     Test_Root & "/missions/" & String (Mission_A) & "/approval-"
     & String (Action_A) & ".apr";

   procedure Overwrite (Path : String; Data : String) is
      Output : File_Type;
   begin
      Create (Output, Out_File, Path);
      Put (Output, Data);
      Close (Output);
   end Overwrite;

   function Slurp (Path : String) return String is
      Input : File_Type;
      Buffer : String (1 .. 512);
      Last : Natural := 0;
      Item : Character;
   begin
      Open (Input, In_File, Path);
      while not End_Of_File (Input) loop
         Get_Immediate (Input, Item);
         Last := Last + 1;
         Buffer (Last) := Item;
      end loop;
      Close (Input);
      return Buffer (1 .. Last);
   end Slurp;

begin
   if Exists (Test_Root) then
      Delete_Tree (Test_Root);
   end if;
   Create_Path (Test_Root & "/missions/" & String (Mission_A));
   Create_Path (Test_Root & "/missions/" & String (Mission_B));

   --  Parent grant: missing, persisted, immutable, round-trip.
   Load_Parent_Grant (Test_Root, Mission_A, Result, Loaded_Grant);
   pragma Assert (Result = Grant_Not_Found);

   Persist_Parent_Grant (Test_Root, Parent, Result);
   pragma Assert (Result = Authority_OK);
   pragma Assert (Exists (Grant_File));

   Persist_Parent_Grant (Test_Root, Parent, Result);
   pragma Assert (Result = Grant_Already_Exists);

   Load_Parent_Grant (Test_Root, Mission_A, Result, Loaded_Grant);
   pragma Assert (Result = Authority_OK);
   pragma Assert (Loaded_Grant = Parent);

   --  Crash residue: a stale temp file is ignored by load and cannot be
   --  promoted by a duplicate persist.
   Overwrite (Grant_File & ".tmp", "junk");
   Load_Parent_Grant (Test_Root, Mission_A, Result, Loaded_Grant);
   pragma Assert (Result = Authority_OK and then Loaded_Grant = Parent);
   Persist_Parent_Grant (Test_Root, Parent, Result);
   pragma Assert (Result = Grant_Already_Exists);

   --  Cross-mission substitution: a grant copied under another mission id
   --  fails the embedded-identity binding.
   declare
      Bytes : constant String := Slurp (Grant_File);
   begin
      Overwrite
        (Test_Root & "/missions/" & String (Mission_B) & "/parent.grant",
         Bytes);
   end;
   Load_Parent_Grant (Test_Root, Mission_B, Result, Loaded_Grant);
   pragma Assert (Result = Authority_Corrupt);

   --  Tamper: truncated, extended, and field-corrupted records refuse.
   declare
      Bytes : constant String := Slurp (Grant_File);
   begin
      Overwrite (Grant_File, Bytes (1 .. Bytes'Last - 2) & ASCII.LF);
      Load_Parent_Grant (Test_Root, Mission_A, Result, Loaded_Grant);
      pragma Assert (Result = Authority_Corrupt);

      Overwrite (Grant_File, Bytes & "x" & ASCII.LF);
      Load_Parent_Grant (Test_Root, Mission_A, Result, Loaded_Grant);
      pragma Assert (Result = Authority_Corrupt);

      declare
         Poisoned : String := Bytes;
      begin
         Poisoned (Poisoned'Last - 1) := 'X';  --  status byte
         Overwrite (Grant_File, Poisoned);
         Load_Parent_Grant (Test_Root, Mission_A, Result, Loaded_Grant);
         pragma Assert (Result = Authority_Corrupt);
      end;

      Overwrite (Grant_File, Bytes);
      Load_Parent_Grant (Test_Root, Mission_A, Result, Loaded_Grant);
      pragma Assert (Result = Authority_OK and then Loaded_Grant = Parent);
   end;

   --  Approval: missing, persisted, immutable, round-trip.
   Load_Approval (Test_Root, Mission_A, Action_A, Result, Loaded_Approval);
   pragma Assert (Result = Approval_Not_Found);

   Persist_Approval (Test_Root, Approval, Result);
   pragma Assert (Result = Authority_OK);
   pragma Assert (Exists (Approval_File));

   Persist_Approval (Test_Root, Approval, Result);
   pragma Assert (Result = Approval_Already_Exists);

   Load_Approval (Test_Root, Mission_A, Action_A, Result, Loaded_Approval);
   pragma Assert (Result = Authority_OK);
   pragma Assert (Loaded_Approval = Approval);

   --  A different action digest has no approval.
   Load_Approval (Test_Root, Mission_A, Action_B, Result, Loaded_Approval);
   pragma Assert (Result = Approval_Not_Found);

   --  Consumption is durable and one-way. The consumed record survives a
   --  reload (simulated restart) and re-issuance stays refused.
   Persist_Consumed_Approval (Test_Root, Approval, Result);
   pragma Assert (Result = Authority_IO_Failure);  --  guard: not consumed

   declare
      Consumed : Approval_Record := Approval;
      Decision : Approval_Decision;
   begin
      Consume_Approval (Consumed, Mission_A, Action_A, 5, Decision);
      pragma Assert (Decision = Approval_Accepted);
      Persist_Consumed_Approval (Test_Root, Consumed, Result);
      pragma Assert (Result = Authority_OK);
   end;

   Load_Approval (Test_Root, Mission_A, Action_A, Result, Loaded_Approval);
   pragma Assert (Result = Authority_OK);
   pragma Assert (Loaded_Approval.Status = Approval_Consumed);

   Persist_Approval (Test_Root, Approval, Result);
   pragma Assert (Result = Approval_Already_Exists);

   --  Consuming a reloaded consumed approval replays and refuses.
   declare
      Decision : Approval_Decision;
   begin
      Consume_Approval
        (Loaded_Approval, Mission_A, Action_A, 5, Decision);
      pragma Assert (Decision = Approval_Replayed);
   end;

   --  Consumed persist for an approval that was never issued refuses.
   declare
      Foreign : constant Approval_Record :=
        (Id            => Apr_A,
         Mission       => Mission_A,
         Action_Digest => Action_B,
         Expires_After => 10_000,
         Status        => Approval_Consumed);
   begin
      Persist_Consumed_Approval (Test_Root, Foreign, Result);
      pragma Assert (Result = Approval_Not_Found);
   end;

   --  Approval tamper refuses.
   declare
      Bytes : constant String := Slurp (Approval_File);
      Poisoned : String := Bytes;
   begin
      Poisoned (Poisoned'Last - 1) := '?';
      Overwrite (Approval_File, Poisoned);
      Load_Approval (Test_Root, Mission_A, Action_A, Result, Loaded_Approval);
      pragma Assert (Result = Authority_Corrupt);
      Overwrite (Approval_File, Bytes);
      Load_Approval (Test_Root, Mission_A, Action_A, Result, Loaded_Approval);
      pragma Assert
        (Result = Authority_OK
         and then Loaded_Approval.Status = Approval_Consumed);
   end;

   Put_Line ("PASS_AUTHORITY_STORE_DURABILITY");
end Nemesis_Authority_Store_Tests;
