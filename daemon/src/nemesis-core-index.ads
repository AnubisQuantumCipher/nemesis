with Nemesis.Core.Ledger;
with Nemesis.Kernel.Types;

package Nemesis.Core.Index with SPARK_Mode => Off is
   use Nemesis.Core.Ledger;
   use Nemesis.Kernel.Types;

   Supported_Schema_Version : constant Positive := 1;

   type Index_Result is
     (Index_OK, Index_Invalid_Input, Index_Unsupported_Schema, Index_Database_Error);

   type Indexed_Mission is record
      Sequence      : Sequence_Number := Sequence_Number'First;
      State         : Mission_State := Draft;
      Ledger_Head   : Digest_Hex := Zero_Digest;
      Source_Digest : Digest_Hex := Zero_Digest;
   end record;

   procedure Initialize
     (Path    : String;
      Result  : out Index_Result;
      Version : out Natural);

   procedure Upsert_Mission
     (Path   : String;
      Id     : Mission_Id;
      Value  : Indexed_Mission;
      Result : out Index_Result);

   procedure Read_Mission
     (Path   : String;
      Id     : Mission_Id;
      Result : out Index_Result;
      Found  : out Boolean;
      Value  : out Indexed_Mission);
end Nemesis.Core.Index;
