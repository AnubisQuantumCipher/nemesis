package body Nemesis.Kernel.Types with SPARK_Mode => On is

   function Encode_State (State : Mission_State) return State_Code is
     (Mission_State'Pos (State));

   function Decode_State (Code : State_Code) return Mission_State is
     (Mission_State'Val (Code));

   function Is_Terminal (State : Mission_State) return Boolean is
     (State in Complete | Blocked_With_Evidence | Cancelled);

   function Next (Value : Sequence_Number) return Sequence_Number is
     (Value + 1);

end Nemesis.Kernel.Types;
