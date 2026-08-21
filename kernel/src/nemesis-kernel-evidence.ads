with Nemesis.Kernel.Types;

package Nemesis.Kernel.Evidence with SPARK_Mode => On is
   use Nemesis.Kernel.Types;

   type Consequence_Class is
     (Informational, Advisory, Decision_Boundary, Safety_Critical);

   type Evidence_Strength is
     (Model_Assertion,
      Direct_Observation,
      Deterministic_Check,
      Independent_Reproduction,
      Formal_Verification,
      Human_Authorization);

   type Verifier_Role is
     (Builder, Independent_Verifier, Formal_Tool, Human_Verifier);

   type Evidence_Status is
     (Evidence_Proposed, Evidence_Accepted, Evidence_Stale, Evidence_Rejected);

   type Claim_Index is range 1 .. 8;

   type Evidence_Record is record
      Mission             : Mission_Id;
      Claim               : Claim_Index;
      Consequence         : Consequence_Class;
      Strength            : Evidence_Strength;
      Verifier            : Verifier_Role;
      Verifier_Authorized : Boolean;
      Source_Digest       : Digest_256;
      Status              : Evidence_Status;
   end record;

   function Acceptable
     (Evidence             : Evidence_Record;
      Current_Source       : Digest_256;
      Required_Consequence : Consequence_Class) return Boolean
   with
     Global => null,
     Post =>
       (if Acceptable'Result then
          Evidence.Status = Evidence_Accepted
          and then Evidence.Source_Digest = Current_Source
          and then Evidence.Verifier_Authorized
          and then Evidence.Consequence >= Required_Consequence
          and then Evidence.Strength /= Model_Assertion);
end Nemesis.Kernel.Evidence;
