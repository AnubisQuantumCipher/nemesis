with Nemesis.Kernel.Transitions;
with Nemesis.Kernel.Types;

package Nemesis.Kernel.Missions with SPARK_Mode => On is

   use Nemesis.Kernel.Transitions;
   use Nemesis.Kernel.Types;

   type Mission_Record is private;

   function Create return Mission_Record
   with
     Global => null,
     Post =>
       State_Of (Create'Result) = Draft
       and then Sequence_Of (Create'Result) = Sequence_Number'First;

   function Restore
     (State : Mission_State; Sequence : Sequence_Number) return Mission_Record
   with
     Global => null,
     Post =>
       State_Of (Restore'Result) = State
       and then Sequence_Of (Restore'Result) = Sequence;

   function State_Of (Mission : Mission_Record) return Mission_State
   with Global => null;

   function Sequence_Of (Mission : Mission_Record) return Sequence_Number
   with Global => null;

   procedure Apply
     (Mission  : in out Mission_Record;
      Target   : Mission_State;
      Decision : out Transition_Decision)
   with
     Global => null,
     Post =>
       (if Decision = Accepted then
          State_Of (Mission) = Target
          and then Sequence_Of (Mission) = Sequence_Of (Mission'Old) + 1
        else
          State_Of (Mission) = State_Of (Mission'Old)
          and then Sequence_Of (Mission) = Sequence_Of (Mission'Old));

   procedure Commit_Event
     (Mission : in out Mission_Record; Decision : out Transition_Decision)
   with
     Global => null,
     Post =>
       State_Of (Mission) = State_Of (Mission'Old)
       and then
         (if Decision = Accepted then
            Sequence_Of (Mission) = Sequence_Of (Mission'Old) + 1
          else Sequence_Of (Mission) = Sequence_Of (Mission'Old));

private
   type Mission_Record is record
      State_Value    : Mission_State := Draft;
      Sequence_Value : Sequence_Number := Sequence_Number'First;
   end record;
end Nemesis.Kernel.Missions;
