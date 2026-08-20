package body Nemesis.Kernel.Types with SPARK_Mode => On is

   function Is_Terminal (State : Mission_State) return Boolean is
     (State in Complete | Blocked_With_Evidence | Cancelled);

   function Next (Value : Sequence_Number) return Sequence_Number is
     (Value + 1);

end Nemesis.Kernel.Types;
