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
      return
        Parent.Status = Active
        and then Child.Status = Active
        and then Child.Mission = Parent.Mission
        and then Child.Resource = Parent.Resource
        and then Child.Scope = Parent.Scope
        and then Child.Expires_After <= Parent.Expires_After
        and then Child.Maximum_Bytes <= Parent.Maximum_Bytes
        and then
          (for all Operation in Operation_Kind =>
             (if Child.Operations (Operation) then
                  Parent.Operations (Operation)));
   end Is_Attenuation;

   function Derive_Child_Grant
     (Parent        : Capability_Grant;
      Child_Id      : Capability_Id;
      Operation     : Operation_Kind;
      Expires_After : Sequence_Number;
      Maximum_Bytes : Natural) return Capability_Grant
   is
     ((Id            => Child_Id,
       Mission       => Parent.Mission,
       Subject       => Parent.Subject,
       Resource      => Parent.Resource,
       Operations    => [for Item in Operation_Kind => Item = Operation],
       Scope         => Parent.Scope,
       Expires_After => Expires_After,
       Maximum_Bytes => Maximum_Bytes,
       Status        => Active));

end Nemesis.Kernel.Capabilities;
