with Nemesis.Kernel.Types;

package Nemesis.Kernel.Capabilities with SPARK_Mode => On is
   use Nemesis.Kernel.Types;

   type Resource_Kind is
     (Filesystem, Process, Network, Git, Secret, Receipt_Signing);

   type Operation_Kind is
     (Read_Data,
      Create_Data,
      Modify_Data,
      Delete_Data,
      Execute_Process,
      Connect_Network,
      Create_Commit,
      Push_Remote,
      Merge_Remote,
      Use_Secret);

   type Operation_Set is array (Operation_Kind) of Boolean;

   type Grant_Status is (Active, Revoked);

   type Capability_Grant is record
      Id            : Capability_Id;
      Mission       : Mission_Id;
      Subject       : Worker_Id;
      Resource      : Resource_Kind;
      Operations    : Operation_Set;
      Scope         : Digest_256;
      Expires_After : Sequence_Number;
      Maximum_Bytes : Natural;
      Status        : Grant_Status;
   end record;

   type Action_Request is record
      Mission         : Mission_Id;
      Subject         : Worker_Id;
      Resource        : Resource_Kind;
      Operation       : Operation_Kind;
      Scope           : Digest_256;
      Estimated_Bytes : Natural;
      Digest          : Digest_256;
   end record;

   function Authorize
     (Grant            : Capability_Grant;
      Action           : Action_Request;
      Current_Sequence : Sequence_Number) return Authority_Decision
   with
     Global => null,
     Post =>
       (if Authorize'Result = Authorized then
          Grant.Status = Active
          and then Current_Sequence <= Grant.Expires_After
          and then Grant.Mission = Action.Mission
          and then Grant.Subject = Action.Subject
          and then Grant.Resource = Action.Resource
          and then Grant.Scope = Action.Scope
          and then Grant.Operations (Action.Operation)
          and then Action.Estimated_Bytes <= Grant.Maximum_Bytes);

   function Is_Attenuation
     (Parent : Capability_Grant; Child : Capability_Grant) return Boolean
   with
     Global => null,
     Post =>
       Is_Attenuation'Result =
         (Parent.Status = Active
          and then Child.Status = Active
          and then Child.Mission = Parent.Mission
          and then Child.Resource = Parent.Resource
          and then Child.Scope = Parent.Scope
          and then Child.Expires_After <= Parent.Expires_After
          and then Child.Maximum_Bytes <= Parent.Maximum_Bytes
          and then
            (for all Operation in Operation_Kind =>
               (if Child.Operations (Operation) then
                    Parent.Operations (Operation))));

   function Derive_Child_Grant
     (Parent        : Capability_Grant;
      Child_Id      : Capability_Id;
      Operation     : Operation_Kind;
      Expires_After : Sequence_Number;
      Maximum_Bytes : Natural) return Capability_Grant
   with
     Global => null,
     Pre  =>
       Parent.Status = Active
       and then Parent.Operations (Operation)
       and then Expires_After <= Parent.Expires_After
       and then Maximum_Bytes <= Parent.Maximum_Bytes,
     Post =>
       Is_Attenuation (Parent, Derive_Child_Grant'Result)
       and then Derive_Child_Grant'Result.Id = Child_Id
       and then Derive_Child_Grant'Result.Mission = Parent.Mission
       and then Derive_Child_Grant'Result.Subject = Parent.Subject
       and then Derive_Child_Grant'Result.Resource = Parent.Resource
       and then Derive_Child_Grant'Result.Scope = Parent.Scope
       and then Derive_Child_Grant'Result.Status = Active
       and then Derive_Child_Grant'Result.Expires_After = Expires_After
       and then Derive_Child_Grant'Result.Maximum_Bytes = Maximum_Bytes
       and then
         (for all Item in Operation_Kind =>
            Derive_Child_Grant'Result.Operations (Item) = (Item = Operation));
end Nemesis.Kernel.Capabilities;
