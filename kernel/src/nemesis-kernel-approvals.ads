with Nemesis.Kernel.Types;

package Nemesis.Kernel.Approvals with SPARK_Mode => On is
   use Nemesis.Kernel.Types;

   type Approval_Status is
     (Approval_Active, Approval_Consumed, Approval_Revoked);

   type Approval_Record is record
      Id            : Approval_Id;
      Mission       : Mission_Id;
      Action_Digest : Digest_256;
      Expires_After : Sequence_Number;
      Status        : Approval_Status;
   end record;

   type Approval_Decision is
     (Approval_Accepted,
      Approval_Replayed,
      Approval_Was_Revoked,
      Approval_Expired,
      Approval_Mission_Mismatch,
      Approval_Action_Mismatch);

   procedure Consume_Approval
     (Approval         : in out Approval_Record;
      Mission          : Mission_Id;
      Action_Digest    : Digest_256;
      Current_Sequence : Sequence_Number;
      Decision         : out Approval_Decision)
   with
     Global => null,
     Post =>
       (if Decision = Approval_Accepted then
          Approval.Status = Approval_Consumed
          and then Approval.Mission = Mission
          and then Approval.Action_Digest = Action_Digest
          and then Current_Sequence <= Approval.Expires_After
        else Approval = Approval'Old);
end Nemesis.Kernel.Approvals;
