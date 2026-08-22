with Nemesis.Kernel.Approvals;
with Nemesis.Kernel.Capabilities;
with Nemesis.Kernel.Types;

package Nemesis.Core.Authority_Store with SPARK_Mode => Off is
   use Nemesis.Kernel.Approvals;
   use Nemesis.Kernel.Capabilities;
   use Nemesis.Kernel.Types;

   type Authority_Status is
     (Authority_OK,
      Grant_Already_Exists,
      Grant_Not_Found,
      Approval_Already_Exists,
      Approval_Not_Found,
      Authority_Corrupt,
      Authority_IO_Failure);

   --  One immutable parent grant per mission. Persisting refuses to
   --  overwrite an existing record so issued authority cannot be widened
   --  by re-issuance.
   procedure Persist_Parent_Grant
     (Home   : String;
      Grant  : Capability_Grant;
      Result : out Authority_Status);

   procedure Load_Parent_Grant
     (Home    : String;
      Mission : Mission_Id;
      Result  : out Authority_Status;
      Grant   : out Capability_Grant);

   --  One approval per mission and exact action digest. Persisting refuses
   --  to overwrite an existing record so a consumed approval can never be
   --  reset to active through re-issuance.
   procedure Persist_Approval
     (Home     : String;
      Approval : Approval_Record;
      Result   : out Authority_Status);

   procedure Load_Approval
     (Home          : String;
      Mission       : Mission_Id;
      Action_Digest : Digest_256;
      Result        : out Authority_Status;
      Approval      : out Approval_Record);

   --  Durably replaces the stored approval record with its consumed form.
   --  Called after the SPARK kernel accepts consumption and before the
   --  authorization event is committed, so a crash between the two burns
   --  the approval (fail closed) rather than permitting replay.
   procedure Persist_Consumed_Approval
     (Home     : String;
      Approval : Approval_Record;
      Result   : out Authority_Status);
end Nemesis.Core.Authority_Store;
