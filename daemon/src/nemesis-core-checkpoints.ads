with Nemesis.Core.Ledger;
with Nemesis.Kernel.Types;

package Nemesis.Core.Checkpoints with SPARK_Mode => Off is
   use Nemesis.Core.Ledger;
   use Nemesis.Kernel.Types;

   type Checkpoint_Record is record
      Sequence      : Sequence_Number := Sequence_Number'First;
      State         : Mission_State := Draft;
      Ledger_Head   : Digest_Hex := Zero_Digest;
      Source_Digest : Digest_Hex := Zero_Digest;
   end record;

   type Checkpoint_Write_Result is
     (Checkpoint_Committed,
      Checkpoint_Invalid_Input,
      Checkpoint_Write_Failed,
      Checkpoint_Sync_Failed,
      Checkpoint_Close_Failed,
      Checkpoint_Rename_Failed);

   type Checkpoint_Read_Result is
     (Checkpoint_Valid,
      Checkpoint_Missing,
      Checkpoint_Corrupt,
      Checkpoint_Unsupported_Version,
      Checkpoint_Read_Failed);

   procedure Write_Checkpoint
     (Path              : String;
      Value             : Checkpoint_Record;
      Result            : out Checkpoint_Write_Result;
      Checkpoint_Digest : out Digest_Hex);

   procedure Read_Checkpoint
     (Path   : String;
      Result : out Checkpoint_Read_Result;
      Value  : out Checkpoint_Record);
end Nemesis.Core.Checkpoints;
