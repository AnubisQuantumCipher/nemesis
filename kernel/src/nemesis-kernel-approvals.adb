package body Nemesis.Kernel.Approvals with SPARK_Mode => On is

   procedure Consume_Approval
     (Approval         : in out Approval_Record;
      Mission          : Mission_Id;
      Action_Digest    : Digest_256;
      Current_Sequence : Sequence_Number;
      Decision         : out Approval_Decision)
   is
   begin
      case Approval.Status is
         when Approval_Consumed =>
            Decision := Approval_Replayed;
            return;
         when Approval_Revoked =>
            Decision := Approval_Was_Revoked;
            return;
         when Approval_Active =>
            null;
      end case;

      if Current_Sequence > Approval.Expires_After then
         Decision := Approval_Expired;
      elsif Mission /= Approval.Mission then
         Decision := Approval_Mission_Mismatch;
      elsif Action_Digest /= Approval.Action_Digest then
         Decision := Approval_Action_Mismatch;
      else
         Approval.Status := Approval_Consumed;
         Decision := Approval_Accepted;
      end if;
   end Consume_Approval;

end Nemesis.Kernel.Approvals;
