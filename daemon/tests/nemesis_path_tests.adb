with Ada.Directories;
with Ada.Text_IO;
with Interfaces.C;
with Interfaces.C.Strings;
with GNAT.OS_Lib;
with Nemesis.Core.Paths;

procedure Nemesis_Path_Tests is
   use Ada.Directories;
   use Ada.Text_IO;
   use Nemesis.Core.Paths;
   use type Interfaces.C.int;

   function C_Symlink
     (Target : Interfaces.C.Strings.chars_ptr;
      Link   : Interfaces.C.Strings.chars_ptr) return Interfaces.C.int
   with Import, Convention => C, External_Name => "symlink";

   procedure Make_Symlink (Target : String; Link : String) is
      Target_C : Interfaces.C.Strings.chars_ptr :=
        Interfaces.C.Strings.New_String (Target);
      Link_C : Interfaces.C.Strings.chars_ptr :=
        Interfaces.C.Strings.New_String (Link);
      Code : Interfaces.C.int;
   begin
      Code := C_Symlink (Target_C, Link_C);
      Interfaces.C.Strings.Free (Target_C);
      Interfaces.C.Strings.Free (Link_C);
      pragma Assert (Code = 0);
   end Make_Symlink;

   Test_Dir : constant String := "build/test-paths";
   Root_Dir : constant String := Test_Dir & "/root";
   Outside_Dir : constant String := Test_Dir & "/outside";
   Inside_File : constant String := Root_Dir & "/inside.txt";
   Outside_File : constant String := Outside_Dir & "/outside.txt";
   Escape_Link : constant String := Root_Dir & "/escape.txt";
   File : File_Type;

begin
   if GNAT.OS_Lib.Is_Symbolic_Link (Escape_Link) or else Exists (Escape_Link)
   then
      Delete_File (Escape_Link);
   end if;
   if Exists (Test_Dir) then
      Delete_Tree (Test_Dir);
   end if;
   Create_Path (Root_Dir);
   Create_Path (Outside_Dir);
   Create (File, Out_File, Inside_File);
   Put (File, "inside");
   Close (File);
   Create (File, Out_File, Outside_File);
   Put (File, "outside");
   Close (File);
   Make_Symlink (Full_Name (Outside_File), Escape_Link);

   pragma Assert
     (Authorize_Existing (Full_Name (Root_Dir), Full_Name (Inside_File)) =
      Path_Authorized);
   pragma Assert
     (Authorize_Existing (Full_Name (Root_Dir), Full_Name (Outside_File)) =
      Path_Outside_Root);
   pragma Assert
     (Authorize_Existing
        (Full_Name (Root_Dir), Full_Name (Root_Dir) & "/escape.txt") =
      Path_Symlink_Escape);
   pragma Assert
     (Authorize_Existing
        (Full_Name (Root_Dir),
         Full_Name (Root_Dir) & "/../outside/outside.txt") =
      Path_Traversal);
   pragma Assert
     (Authorize_Existing (Full_Name (Root_Dir), "relative.txt") =
      Path_Not_Absolute);
   pragma Assert
     (Authorize_Existing (Full_Name (Root_Dir), Full_Name (Root_Dir)) =
      Path_Special_File);

   pragma Assert
     (Authorize_Create
        (Full_Name (Root_Dir), Full_Name (Root_Dir) & "/created.txt") =
      Path_Authorized);
   pragma Assert
     (Authorize_Create
        (Full_Name (Root_Dir),
         Full_Name (Root_Dir) & "/../outside/created.txt") =
      Path_Traversal);
   pragma Assert
     (Authorize_Create
        (Full_Name (Root_Dir), Full_Name (Outside_Dir) & "/created.txt") =
      Path_Outside_Root);

   Put_Line ("PASS_PATH_TRAVERSAL_AND_SYMLINK_GATES");
end Nemesis_Path_Tests;
