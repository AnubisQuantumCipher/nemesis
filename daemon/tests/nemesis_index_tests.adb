with Ada.Directories;
with Ada.Text_IO;
with Nemesis.Core.Index;
with Nemesis.Core.Ledger;
with Nemesis.Kernel.Types;

procedure Nemesis_Index_Tests is
   use Ada.Directories;
   use Ada.Text_IO;
   use Nemesis.Core.Index;
   use Nemesis.Core.Ledger;
   use Nemesis.Kernel.Types;

   Test_Dir : constant String := "build/test-index";
   DB_Path  : constant String := Test_Dir & "/missions.sqlite3";
   Id       : constant Mission_Id := "mis_0000000000000000000000";

   Status  : Index_Result;
   Version : Natural := 0;
   Found   : Boolean := False;
   Value   : Indexed_Mission;

begin
   if Exists (Test_Dir) then
      Delete_Tree (Test_Dir);
   end if;
   Create_Path (Test_Dir);

   Initialize (DB_Path, Status, Version);
   pragma Assert (Status = Index_OK);
   pragma Assert (Version = Supported_Schema_Version);
   pragma Assert (Exists (DB_Path));

   Read_Mission (DB_Path, Id, Status, Found, Value);
   pragma Assert (Status = Index_OK);
   pragma Assert (not Found);

   Upsert_Mission
     (Path => DB_Path,
      Id => Id,
      Value =>
        (Sequence      => 4,
         State         => Running,
         Ledger_Head   => Digest ("head-v1"),
         Source_Digest => Digest ("source-v1")),
      Result => Status);
   pragma Assert (Status = Index_OK);

   Read_Mission (DB_Path, Id, Status, Found, Value);
   pragma Assert (Status = Index_OK);
   pragma Assert (Found);
   pragma Assert (Value.Sequence = 4);
   pragma Assert (Value.State = Running);
   pragma Assert (Value.Ledger_Head = Digest ("head-v1"));

   Upsert_Mission
     (Path => DB_Path,
      Id => Id,
      Value =>
        (Sequence      => 5,
         State         => Verifying,
         Ledger_Head   => Digest ("head-v2"),
         Source_Digest => Digest ("source-v2")),
      Result => Status);
   pragma Assert (Status = Index_OK);
   Read_Mission (DB_Path, Id, Status, Found, Value);
   pragma Assert (Status = Index_OK);
   pragma Assert (Found);
   pragma Assert (Value.Sequence = 5);
   pragma Assert (Value.State = Verifying);
   pragma Assert (Value.Source_Digest = Digest ("source-v2"));

   Put_Line ("PASS_SQLITE_INDEX_MIGRATION");
end Nemesis_Index_Tests;
