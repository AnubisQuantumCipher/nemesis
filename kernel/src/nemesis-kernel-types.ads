package Nemesis.Kernel.Types with SPARK_Mode => On is

   subtype Mission_Id is String (1 .. 26);

   type Mission_State is
     (Draft,
      Contract_Compiled,
      Awaiting_Authorization,
      Planning,
      Running,
      Waiting_Approval,
      Recovering,
      Verifying,
      Complete,
      Blocked_With_Evidence,
      Cancelled);

   type State_Code is range
     Mission_State'Pos (Mission_State'First)
       .. Mission_State'Pos (Mission_State'Last);

   function Encode_State (State : Mission_State) return State_Code
   with
     Global => null,
     Post => Encode_State'Result = Mission_State'Pos (State);

   function Decode_State (Code : State_Code) return Mission_State
   with
     Global => null,
     Post => Mission_State'Pos (Decode_State'Result) = Code;

   type Authority_Decision is
     (Authorized,
      Refused_Capability,
      Refused_Policy,
      Refused_Budget,
      Refused_Approval,
      Requires_Approval,
      Requires_Stronger_Sandbox,
      Requires_Independent_Review);

   type Sequence_Number is range 0 .. 2 ** 63 - 1;

   function Is_Terminal (State : Mission_State) return Boolean
   with
     Global => null,
     Post =>
       Is_Terminal'Result =
         (State in Complete | Blocked_With_Evidence | Cancelled);

   function Next (Value : Sequence_Number) return Sequence_Number
   with
     Global => null,
     Pre  => Value < Sequence_Number'Last,
     Post => Next'Result = Value + 1;
end Nemesis.Kernel.Types;
