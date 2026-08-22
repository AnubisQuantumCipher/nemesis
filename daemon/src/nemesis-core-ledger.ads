with Nemesis.Kernel.Types;

package Nemesis.Core.Ledger with SPARK_Mode => Off is
   use Nemesis.Kernel.Types;

   subtype Digest_Hex is String (1 .. 64);
   Zero_Digest : constant Digest_Hex := [others => '0'];

   type Event_Kind is
     (Mission_Created,
      Contract_Compiled_Event,
      Contract_Authorized,
      State_Transitioned,
      Action_Authorized,
      Action_Completed,
      Evidence_Accepted,
      Completion_Proposed,
      Mission_Completed,
      Mission_Blocked);

   type Append_Result is
     (Committed,
      Invalid_Input,
      Invalid_Existing_Ledger,
      Sequence_Exhausted,
      Write_Failed,
      Sync_Failed,
      Close_Failed);

   type Recovery_Status is
     (Empty,
      Recovered_Valid,
      Truncated_Tail,
      Corrupt,
      Unsupported_Version,
      Read_Failed);

   type Recovery_Result is record
      Status   : Recovery_Status := Empty;
      Sequence : Sequence_Number := Sequence_Number'First;
      State    : Mission_State := Draft;
      Head     : Digest_Hex := Zero_Digest;
      First_Payload : Digest_Hex := Zero_Digest;
      Last_Payload  : Digest_Hex := Zero_Digest;
      Source        : Digest_Hex := Zero_Digest;
   end record;

   function Digest (Value : String) return Digest_Hex;

   procedure Append
     (Path           : String;
      Kind           : Event_Kind;
      State          : Mission_State;
      Source_Digest  : Digest_Hex;
      Payload_Digest : Digest_Hex;
      Result         : out Append_Result;
      Event_Digest   : out Digest_Hex);

   procedure Recover (Path : String; Result : out Recovery_Result);
end Nemesis.Core.Ledger;
