package Nemesis.Kernel.Completion with SPARK_Mode => On is
   type Claim_Status is
     (Not_Required,
      Required_Missing,
      Supported_Current,
      Supported_Stale,
      Conflicting,
      Human_Decision_Pending);

   type Claim_Status_Array is array (Positive range <>) of Claim_Status;

   type Completion_Decision is
     (Completion_Accepted,
      Completion_Returned_To_Running,
      Completion_Blocked);

   function Evaluate
     (Claims          : Claim_Status_Array;
      Has_Blocker     : Boolean;
      Worker_Proposed : Boolean) return Completion_Decision
   with
     Global => null,
     Post =>
       (if Evaluate'Result = Completion_Accepted then
          Worker_Proposed
          and then not Has_Blocker
          and then
            (for all Index in Claims'Range =>
               Claims (Index) in Not_Required | Supported_Current));
end Nemesis.Kernel.Completion;
