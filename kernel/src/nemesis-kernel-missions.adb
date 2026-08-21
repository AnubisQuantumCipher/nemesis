package body Nemesis.Kernel.Missions with SPARK_Mode => On is

   function Create return Mission_Record is
     (State_Value    => Draft,
      Sequence_Value => Sequence_Number'First);

   function Restore
     (State : Mission_State; Sequence : Sequence_Number) return Mission_Record
   is
     (State_Value => State, Sequence_Value => Sequence);

   function State_Of (Mission : Mission_Record) return Mission_State is
     (Mission.State_Value);

   function Sequence_Of (Mission : Mission_Record) return Sequence_Number is
     (Mission.Sequence_Value);

   procedure Apply
     (Mission  : in out Mission_Record;
      Target   : Mission_State;
      Decision : out Transition_Decision)
   is
   begin
      if Is_Terminal (Mission.State_Value) then
         Decision := Refused_Terminal_State;
      elsif not Allowed (Mission.State_Value, Target) then
         Decision := Refused_Illegal_Transition;
      elsif Mission.Sequence_Value = Sequence_Number'Last then
         Decision := Refused_Sequence_Exhausted;
      else
         Mission.State_Value := Target;
         Mission.Sequence_Value := Next (Mission.Sequence_Value);
         Decision := Accepted;
      end if;
   end Apply;

   procedure Commit_Event
     (Mission : in out Mission_Record; Decision : out Transition_Decision)
   is
   begin
      if Is_Terminal (Mission.State_Value) then
         Decision := Refused_Terminal_State;
      elsif Mission.Sequence_Value = Sequence_Number'Last then
         Decision := Refused_Sequence_Exhausted;
      else
         Mission.Sequence_Value := Next (Mission.Sequence_Value);
         Decision := Accepted;
      end if;
   end Commit_Event;

end Nemesis.Kernel.Missions;
