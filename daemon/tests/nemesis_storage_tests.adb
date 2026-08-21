with Ada.Directories;
with Ada.Strings.Unbounded;
with Ada.Text_IO;
with Nemesis.Core.Checkpoints;
with Nemesis.Core.Ledger;
with Nemesis.Core.Objects;
with Nemesis.Kernel.Types;

procedure Nemesis_Storage_Tests is
   use Ada.Directories;
   use Ada.Strings.Unbounded;
   use Ada.Text_IO;
   use Nemesis.Core.Checkpoints;
   use Nemesis.Core.Ledger;
   use Nemesis.Core.Objects;
   use Nemesis.Kernel.Types;

   Test_Root       : constant String := "build/test-storage";
   Checkpoint_Path : constant String := Test_Root & "/checkpoint.ncp";
   Alpha_Digest    : constant Digest_Hex :=
     "8ed3f6ad685b959ead7022518e1af76cd816f8e8ec7ccdda1ed4018e8f2223f8";

   Store_Status : Store_Result;
   Load_Status  : Load_Result;
   Object_Id    : Digest_Hex;
   Loaded       : Unbounded_String;
   Save_Status  : Checkpoint_Write_Result;
   Read_Status  : Checkpoint_Read_Result;
   Saved_Digest : Digest_Hex;
   Loaded_Checkpoint : Checkpoint_Record;

begin
   if Exists (Test_Root) then
      Delete_Tree (Test_Root);
   end if;
   Create_Path (Test_Root);

   Put
     (Root => Test_Root, Data => "alpha", Result => Store_Status,
      Object_Digest => Object_Id);
   pragma Assert (Store_Status = Stored);
   pragma Assert (Object_Id = Alpha_Digest);
   pragma Assert (Exists (Path_Of (Test_Root, Object_Id)));

   Put
     (Root => Test_Root, Data => "alpha", Result => Store_Status,
      Object_Digest => Object_Id);
   pragma Assert (Store_Status = Already_Present);

   Get
     (Root => Test_Root, Object_Digest => Object_Id,
      Result => Load_Status, Data => Loaded);
   pragma Assert (Load_Status = Loaded_Valid);
   pragma Assert (To_String (Loaded) = "alpha");

   declare
      Tampered : File_Type;
   begin
      Create (Tampered, Out_File, Path_Of (Test_Root, Object_Id));
      Put (Tampered, "omega");
      Close (Tampered);
   end;
   Get
     (Root => Test_Root, Object_Digest => Object_Id,
      Result => Load_Status, Data => Loaded);
   pragma Assert (Load_Status = Integrity_Failed);

   Put
     (Root => Test_Root, Data => "alpha", Result => Store_Status,
      Object_Digest => Object_Id);
   pragma Assert (Store_Status = Collision_Detected);

   Write_Checkpoint
     (Path => Checkpoint_Path,
      Value =>
        (Sequence      => 2,
         State         => Running,
         Ledger_Head   => Digest ("ledger-head"),
         Source_Digest => Digest ("source-v1")),
      Result => Save_Status,
      Checkpoint_Digest => Saved_Digest);
   pragma Assert (Save_Status = Checkpoint_Committed);

   Read_Checkpoint
     (Path => Checkpoint_Path,
      Result => Read_Status,
      Value => Loaded_Checkpoint);
   pragma Assert (Read_Status = Checkpoint_Valid);
   pragma Assert (Loaded_Checkpoint.Sequence = 2);
   pragma Assert (Loaded_Checkpoint.State = Running);
   pragma Assert (Loaded_Checkpoint.Ledger_Head = Digest ("ledger-head"));

   Write_Checkpoint
     (Path => Checkpoint_Path,
      Value =>
        (Sequence      => 3,
         State         => Verifying,
         Ledger_Head   => Digest ("ledger-head-2"),
         Source_Digest => Digest ("source-v2")),
      Result => Save_Status,
      Checkpoint_Digest => Saved_Digest);
   pragma Assert (Save_Status = Checkpoint_Committed);
   Read_Checkpoint
     (Path => Checkpoint_Path,
      Result => Read_Status,
      Value => Loaded_Checkpoint);
   pragma Assert (Read_Status = Checkpoint_Valid);
   pragma Assert (Loaded_Checkpoint.Sequence = 3);
   pragma Assert (Loaded_Checkpoint.State = Verifying);
   pragma Assert (Loaded_Checkpoint.Source_Digest = Digest ("source-v2"));

   declare
      Tampered : File_Type;
   begin
      Open (Tampered, Append_File, Checkpoint_Path);
      Put (Tampered, "x");
      Close (Tampered);
   end;
   Read_Checkpoint
     (Path => Checkpoint_Path,
      Result => Read_Status,
      Value => Loaded_Checkpoint);
   pragma Assert (Read_Status = Checkpoint_Corrupt);

   Put_Line ("PASS_DURABLE_STORAGE");
end Nemesis_Storage_Tests;
