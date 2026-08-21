package body Nemesis.Kernel.Evidence with SPARK_Mode => On is

   function Acceptable
     (Evidence             : Evidence_Record;
      Current_Source       : Digest_256;
      Required_Consequence : Consequence_Class) return Boolean
   is
   begin
      if Evidence.Status /= Evidence_Accepted
        or else Evidence.Source_Digest /= Current_Source
        or else not Evidence.Verifier_Authorized
        or else Evidence.Consequence < Required_Consequence
        or else Evidence.Strength = Model_Assertion
      then
         return False;
      end if;

      case Required_Consequence is
         when Informational | Advisory =>
            return Evidence.Strength >= Direct_Observation;
         when Decision_Boundary =>
            return
              Evidence.Strength >= Deterministic_Check
              and then Evidence.Verifier /= Builder;
         when Safety_Critical =>
            return
              Evidence.Strength >= Independent_Reproduction
              and then Evidence.Verifier /= Builder;
      end case;
   end Acceptable;

end Nemesis.Kernel.Evidence;
