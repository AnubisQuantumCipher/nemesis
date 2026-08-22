with Ada.Text_IO;
with Nemesis.Kernel.Approvals;
with Nemesis.Kernel.Budgets;
with Nemesis.Kernel.Capabilities;
with Nemesis.Kernel.Completion;
with Nemesis.Kernel.Evidence;
with Nemesis.Kernel.Types;

procedure Nemesis_Authority_Tests is
   use Ada.Text_IO;
   use Nemesis.Kernel.Approvals;
   use Nemesis.Kernel.Budgets;
   use Nemesis.Kernel.Capabilities;
   use Nemesis.Kernel.Completion;
   use Nemesis.Kernel.Evidence;
   use Nemesis.Kernel.Types;

   Mission_A : constant Mission_Id := "mis_0000000000000000000000";
   Mission_B : constant Mission_Id := "mis_1111111111111111111111";
   Worker_A  : constant Worker_Id := "wrk_0000000000000000000000";
   Worker_B  : constant Worker_Id := "wrk_1111111111111111111111";
   Cap_A     : constant Capability_Id := "cap_0000000000000000000000";
   Approval_A : constant Approval_Id := "apr_0000000000000000000000";
   Scope_A   : constant Digest_256 := [others => 'a'];
   Scope_B   : constant Digest_256 := [others => 'b'];
   Action_A  : constant Digest_256 := [others => 'c'];
   Action_B  : constant Digest_256 := [others => 'd'];
   Source_A  : constant Digest_256 := [others => 'e'];
   Source_B  : constant Digest_256 := [others => 'f'];

   Parent : constant Capability_Grant :=
     (Id             => Cap_A,
      Mission        => Mission_A,
      Subject        => Worker_A,
      Resource       => Filesystem,
      Operations     => [Read_Data | Modify_Data => True, others => False],
      Scope          => Scope_A,
      Expires_After  => 10,
      Maximum_Bytes  => 1_024,
      Status         => Active);
   Action : constant Action_Request :=
     (Mission         => Mission_A,
      Subject         => Worker_A,
      Resource        => Filesystem,
      Operation       => Modify_Data,
      Scope           => Scope_A,
      Estimated_Bytes => 512,
      Digest          => Action_A);
   Child : Capability_Grant :=
     (Id             => "cap_1111111111111111111111",
      Mission        => Mission_A,
      Subject        => Worker_B,
      Resource       => Filesystem,
      Operations     => [Read_Data => True, others => False],
      Scope          => Scope_A,
      Expires_After  => 8,
      Maximum_Bytes  => 256,
      Status         => Active);

   Budget : Budget_Record :=
     (Cost_Microunits => 1_000,
      Tokens          => 10_000,
      Time_Millis     => 5_000,
      Processes       => 4,
      Storage_Bytes   => 8_192,
      Network_Bytes   => 0);
   Request : constant Budget_Request :=
     (Cost_Microunits => 100,
      Tokens          => 500,
      Time_Millis     => 250,
      Processes       => 1,
      Storage_Bytes   => 1_024,
      Network_Bytes   => 0);
   Budget_Result : Budget_Decision;
   Before_Budget : Budget_Record;

   Approval : Approval_Record :=
     (Id            => Approval_A,
      Mission       => Mission_A,
      Action_Digest => Action_A,
      Expires_After => 10,
      Status        => Approval_Active);
   Approval_Result : Approval_Decision;

   Accepted_Evidence : constant Evidence_Record :=
     (Mission             => Mission_A,
      Claim               => 1,
      Consequence         => Decision_Boundary,
      Strength            => Deterministic_Check,
      Verifier            => Independent_Verifier,
      Verifier_Authorized => True,
      Source_Digest       => Source_A,
      Status              => Evidence_Accepted);
   Claims : Claim_Status_Array (1 .. 3) :=
     [Supported_Current, Supported_Current, Supported_Current];

begin
   pragma Assert (Authorize (Parent, Action, 5) = Authorized);
   pragma Assert
     (Authorize
        (Parent, (Action with delta Operation => Delete_Data), 5) =
      Refused_Capability);
   pragma Assert
     (Authorize
        (Parent, (Action with delta Mission => Mission_B), 5) =
      Refused_Capability);
   pragma Assert
     (Authorize
        (Parent, (Action with delta Subject => Worker_B), 5) =
      Refused_Capability);
   pragma Assert
     (Authorize
        (Parent, (Action with delta Scope => Scope_B), 5) =
      Refused_Capability);
   pragma Assert
     (Authorize
        ((Parent with delta Status => Revoked), Action, 5) =
      Refused_Capability);
   pragma Assert (Authorize (Parent, Action, 11) = Refused_Capability);
   pragma Assert
     (Authorize
        (Parent, (Action with delta Estimated_Bytes => 2_048), 5) =
      Refused_Budget);

   pragma Assert (Is_Attenuation (Parent, Child));
   Child.Operations (Delete_Data) := True;
   pragma Assert (not Is_Attenuation (Parent, Child));
   Child.Operations (Delete_Data) := False;
   Child.Scope := Scope_B;
   pragma Assert (not Is_Attenuation (Parent, Child));

   declare
      Derived : constant Capability_Grant :=
        Derive_Child_Grant
          (Parent        => Parent,
           Child_Id      => "cap_2222222222222222222222",
           Operation     => Modify_Data,
           Expires_After => 9,
           Maximum_Bytes => 512);
   begin
      pragma Assert (Is_Attenuation (Parent, Derived));
      pragma Assert (Derived.Subject = Worker_A);
      pragma Assert (Derived.Mission = Mission_A);
      pragma Assert (Derived.Scope = Scope_A);
      pragma Assert (Derived.Operations (Modify_Data));
      pragma Assert (not Derived.Operations (Read_Data));
      pragma Assert (not Derived.Operations (Delete_Data));
      pragma Assert
        (Authorize (Derived, Action, 5) = Authorized);
      --  The derived child expires independently of the parent.
      pragma Assert
        (Authorize (Derived, Action, 10) = Refused_Capability);
      --  The derived child budget binds below the parent budget.
      pragma Assert
        (Authorize
           (Derived, (Action with delta Estimated_Bytes => 513), 5) =
         Refused_Budget);
      --  A subject mismatch still refuses at the derived child.
      pragma Assert
        (Authorize
           (Derived, (Action with delta Subject => Worker_B), 5) =
         Refused_Capability);
   end;

   Consume (Budget, Request, Budget_Result);
   pragma Assert (Budget_Result = Budget_Authorized);
   pragma Assert (Budget.Storage_Bytes = 7_168);
   pragma Assert (Budget.Processes = 3);
   Before_Budget := Budget;
   Consume
     (Budget,
      (Request with delta Storage_Bytes => 16_384),
      Budget_Result);
   pragma Assert (Budget_Result = Budget_Refused);
   pragma Assert (Budget = Before_Budget);
   pragma Assert
     (Can_Allocate
        (Budget,
         (Request with delta Storage_Bytes => 2_048, Processes => 2)));
   pragma Assert
     (not Can_Allocate
        (Budget,
         (Request with delta Network_Bytes => 1)));

   Consume_Approval
     (Approval, Mission_A, Action_A, 5, Approval_Result);
   pragma Assert (Approval_Result = Approval_Accepted);
   pragma Assert (Approval.Status = Approval_Consumed);
   Consume_Approval
     (Approval, Mission_A, Action_A, 5, Approval_Result);
   pragma Assert (Approval_Result = Approval_Replayed);

   Approval.Status := Approval_Active;
   Consume_Approval
     (Approval, Mission_A, Action_B, 5, Approval_Result);
   pragma Assert (Approval_Result = Approval_Action_Mismatch);
   Consume_Approval
     (Approval, Mission_B, Action_A, 5, Approval_Result);
   pragma Assert (Approval_Result = Approval_Mission_Mismatch);
   Consume_Approval
     (Approval, Mission_A, Action_A, 11, Approval_Result);
   pragma Assert (Approval_Result = Approval_Expired);

   pragma Assert
     (Acceptable
        (Accepted_Evidence, Source_A, Decision_Boundary));
   pragma Assert
     (not Acceptable
        ((Accepted_Evidence with delta Source_Digest => Source_B),
         Source_A,
         Decision_Boundary));
   pragma Assert
     (not Acceptable
        ((Accepted_Evidence with delta Verifier => Builder),
         Source_A,
         Decision_Boundary));
   pragma Assert
     (not Acceptable
        ((Accepted_Evidence with delta Strength => Model_Assertion),
         Source_A,
         Decision_Boundary));
   pragma Assert
     (not Acceptable
        ((Accepted_Evidence with delta Status => Evidence_Stale),
         Source_A,
         Decision_Boundary));

   pragma Assert
     (Evaluate (Claims, Has_Blocker => False, Worker_Proposed => True) =
      Completion_Accepted);
   Claims (2) := Required_Missing;
   pragma Assert
     (Evaluate (Claims, Has_Blocker => False, Worker_Proposed => True) =
      Completion_Returned_To_Running);
   Claims (2) := Supported_Current;
   pragma Assert
     (Evaluate (Claims, Has_Blocker => True, Worker_Proposed => True) =
      Completion_Blocked);
   pragma Assert
     (Evaluate (Claims, Has_Blocker => False, Worker_Proposed => False) =
      Completion_Returned_To_Running);

   Put_Line ("PASS_HOSTILE_CONTROLS");
   Put_Line ("PASS_COMPLETION_COURT");
end Nemesis_Authority_Tests;
