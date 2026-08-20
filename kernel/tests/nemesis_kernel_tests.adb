with Ada.Text_IO;
with Nemesis.Kernel.Missions;
with Nemesis.Kernel.Transitions;
with Nemesis.Kernel.Types;

procedure Nemesis_Kernel_Tests is
   use Ada.Text_IO;
   use Nemesis.Kernel.Missions;
   use Nemesis.Kernel.Transitions;
   use Nemesis.Kernel.Types;

   function Expected_Terminal (State : Mission_State) return Boolean is
     (State in Complete | Blocked_With_Evidence | Cancelled);

   function Expected_Allowed
     (Source : Mission_State; Target : Mission_State) return Boolean
   is
     (case Source is
         when Draft                  => Target = Contract_Compiled,
         when Contract_Compiled      => Target = Awaiting_Authorization,
         when Awaiting_Authorization => Target = Planning,
         when Planning               => Target = Running,
         when Running                =>
           Target in Waiting_Approval
                   | Recovering
                   | Verifying
                   | Blocked_With_Evidence
                   | Cancelled,
         when Waiting_Approval       => Target in Running | Cancelled,
         when Recovering             =>
           Target in Running | Blocked_With_Evidence | Cancelled,
         when Verifying              =>
           Target in Running | Complete | Blocked_With_Evidence | Cancelled,
         when Complete | Blocked_With_Evidence | Cancelled => False);

   Mission  : Mission_Record := Create;
   Restored : constant Mission_Record := Restore (Verifying, 42);
   Decision : Transition_Decision;
   Before   : Sequence_Number;

begin
   for State in Mission_State loop
      pragma Assert (Is_Terminal (State) = Expected_Terminal (State));
      pragma Assert (Decode_State (Encode_State (State)) = State);
   end loop;

   for Source in Mission_State loop
      for Target in Mission_State loop
         pragma Assert
           (Allowed (Source, Target) = Expected_Allowed (Source, Target));
      end loop;
   end loop;

   pragma Assert (State_Of (Restored) = Verifying);
   pragma Assert (Sequence_Of (Restored) = 42);

   pragma Assert (Next (Sequence_Number'First) = Sequence_Number'First + 1);
   pragma Assert (Mission_Id'Length = 26);
   pragma Assert (Authority_Decision'First = Authorized);
   pragma Assert (Authority_Decision'Last = Requires_Independent_Review);
   pragma Assert (State_Of (Mission) = Draft);
   pragma Assert (Sequence_Of (Mission) = 0);

   Apply (Mission, Contract_Compiled, Decision);
   pragma Assert (Decision = Accepted);
   pragma Assert (State_Of (Mission) = Contract_Compiled);
   pragma Assert (Sequence_Of (Mission) = 1);

   Before := Sequence_Of (Mission);
   Apply (Mission, Complete, Decision);
   pragma Assert (Decision = Refused_Illegal_Transition);
   pragma Assert (State_Of (Mission) = Contract_Compiled);
   pragma Assert (Sequence_Of (Mission) = Before);

   Apply (Mission, Awaiting_Authorization, Decision);
   Apply (Mission, Planning, Decision);
   Apply (Mission, Running, Decision);
   Before := Sequence_Of (Mission);
   Commit_Event (Mission, Decision);
   pragma Assert (Decision = Accepted);
   pragma Assert (State_Of (Mission) = Running);
   pragma Assert (Sequence_Of (Mission) = Before + 1);
   Apply (Mission, Cancelled, Decision);
   pragma Assert (Decision = Accepted);

   Before := Sequence_Of (Mission);
   Apply (Mission, Running, Decision);
   pragma Assert (Decision = Refused_Terminal_State);
   pragma Assert (State_Of (Mission) = Cancelled);
   pragma Assert (Sequence_Of (Mission) = Before);

   Put_Line ("PASS_KERNEL_TYPES");
   Put_Line ("PASS_MISSION_TRANSITIONS");
end Nemesis_Kernel_Tests;
