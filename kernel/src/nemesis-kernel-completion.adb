package body Nemesis.Kernel.Completion with SPARK_Mode => On is

   function Evaluate
     (Claims          : Claim_Status_Array;
      Has_Blocker     : Boolean;
      Worker_Proposed : Boolean) return Completion_Decision
   is
   begin
      if Has_Blocker
        or else
          (for some Index in Claims'Range =>
             Claims (Index) = Conflicting)
      then
         return Completion_Blocked;
      elsif not Worker_Proposed
        or else
          (for some Index in Claims'Range =>
             Claims (Index) not in Not_Required | Supported_Current)
      then
         return Completion_Returned_To_Running;
      else
         return Completion_Accepted;
      end if;
   end Evaluate;

end Nemesis.Kernel.Completion;
