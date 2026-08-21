package body Nemesis.Kernel.Capabilities with SPARK_Mode => On is

   function Authorize
     (Grant            : Capability_Grant;
      Action           : Action_Request;
      Current_Sequence : Sequence_Number) return Authority_Decision
   is
   begin
      if Grant.Status /= Active
        or else Current_Sequence > Grant.Expires_After
        or else Grant.Mission /= Action.Mission
        or else Grant.Subject /= Action.Subject
        or else Grant.Resource /= Action.Resource
        or else Grant.Scope /= Action.Scope
        or else not Grant.Operations (Action.Operation)
      then
         return Refused_Capability;
      elsif Action.Estimated_Bytes > Grant.Maximum_Bytes then
         return Refused_Budget;
      else
         return Authorized;
      end if;
   end Authorize;

   function Is_Attenuation
     (Parent : Capability_Grant; Child : Capability_Grant) return Boolean
   is
   begin
      if Parent.Status /= Active
        or else Child.Status /= Active
        or else Child.Mission /= Parent.Mission
        or else Child.Resource /= Parent.Resource
        or else Child.Scope /= Parent.Scope
        or else Child.Expires_After > Parent.Expires_After
        or else Child.Maximum_Bytes > Parent.Maximum_Bytes
      then
         return False;
      end if;

      for Operation in Operation_Kind loop
         if Child.Operations (Operation) and then not Parent.Operations (Operation)
         then
            return False;
         end if;
      end loop;
      return True;
   end Is_Attenuation;

end Nemesis.Kernel.Capabilities;
