with Nemesis.Kernel.Types;

package Nemesis.Kernel.Transitions with SPARK_Mode => On is
   pragma Pure;

   use Nemesis.Kernel.Types;

   type Transition_Decision is
     (Accepted,
      Refused_Illegal_Transition,
      Refused_Terminal_State,
      Refused_Sequence_Exhausted);

   function Allowed
     (Source : Mission_State; Target : Mission_State) return Boolean
   with
     Global => null,
     Post => (if Is_Terminal (Source) then not Allowed'Result);
end Nemesis.Kernel.Transitions;
