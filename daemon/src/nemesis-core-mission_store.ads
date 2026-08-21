with Nemesis.Core.Ledger;
with Nemesis.Kernel.Missions;
with Nemesis.Kernel.Types;

package Nemesis.Core.Mission_Store with SPARK_Mode => Off is
   use Nemesis.Kernel.Missions;
   use Nemesis.Core.Ledger;
   use Nemesis.Kernel.Types;

   type Store_Status is
     (Store_OK,
      Mission_Already_Exists,
      Mission_Not_Found,
      Contract_Mismatch,
      Evidence_Stale,
      Store_Corrupt,
      Store_Refused,
      Store_IO_Failure);

   type Mission_Context is record
      Mission          : Mission_Record;
      Id               : Mission_Id;
      Worker           : Worker_Id;
      Contract_Digest  : Digest_256;
      Scope_Digest     : Digest_256;
      Current_Source   : Digest_256;
      Ledger_Head      : Digest_256;
      Build_Source     : Digest_256;
      Build_Evidence   : Digest_256;
      Test_Source      : Digest_256;
      Test_Evidence    : Digest_256;
   end record;

   procedure Create_Mission
     (Home             : String;
      Id               : Mission_Id;
      Worker           : Worker_Id;
      Contract_Digest  : Digest_256;
      Scope_Digest     : Digest_256;
      Initial_Source   : Digest_256;
      Result           : out Store_Status;
      Context          : out Mission_Context);

   procedure Load_Mission
     (Home    : String;
      Id      : Mission_Id;
      Result  : out Store_Status;
      Context : out Mission_Context);

   procedure Commit_Transition
     (Home       : String;
      Context    : in out Mission_Context;
      Target     : Mission_State;
      Kind       : Event_Kind;
      Payload    : Digest_256;
      Result     : out Store_Status);

   procedure Commit_Event
     (Home       : String;
      Context    : in out Mission_Context;
      Kind       : Event_Kind;
      Payload    : Digest_256;
      Result     : out Store_Status);

   procedure Record_Source
     (Home       : String;
      Context    : in out Mission_Context;
      New_Source : Digest_256;
      Action     : Digest_256;
      Result     : out Store_Status);

   procedure Record_Evidence
     (Home      : String;
      Context   : in out Mission_Context;
      Claim     : String;
      Source    : Digest_256;
      Evidence  : Digest_256;
      Result    : out Store_Status);

   function Claims_Current (Context : Mission_Context) return Boolean;

   function Ledger_Path (Home : String; Id : Mission_Id) return String;
end Nemesis.Core.Mission_Store;
