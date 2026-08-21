with Ada.Directories;
with Ada.Streams;
with Ada.Streams.Stream_IO;
with Ada.Text_IO;
with Nemesis.Core.Ledger;
with Nemesis.Kernel.Types;

procedure Nemesis_Ledger_Tests is
   use Ada.Directories;
   use Ada.Streams;
   use Ada.Text_IO;
   use Nemesis.Core.Ledger;
   use Nemesis.Kernel.Types;

   Test_Dir       : constant String := "build/test-ledger";
   Valid_Path     : constant String := Test_Dir & "/valid.ledger";
   Corrupt_Path   : constant String := Test_Dir & "/corrupt.ledger";
   Truncated_Path : constant String := Test_Dir & "/truncated.ledger";
   Partial_First_Path : constant String := Test_Dir & "/partial-first.ledger";
   Reordered_Path : constant String := Test_Dir & "/reordered.ledger";
   Duplicate_Path : constant String := Test_Dir & "/duplicate.ledger";
   Version_Path   : constant String := Test_Dir & "/version.ledger";

   procedure Delete_If_Present (Path : String) is
   begin
      if Exists (Path) then
         Delete_File (Path);
      end if;
   end Delete_If_Present;

   procedure Mutate_Byte (Path : String; Offset : Positive) is
      package SIO renames Ada.Streams.Stream_IO;
      Input  : SIO.File_Type;
      Output : SIO.File_Type;
   begin
      SIO.Open (Input, SIO.In_File, Path);
      declare
         Length : constant Stream_Element_Count :=
           Stream_Element_Count (SIO.Size (Input));
         Data : Stream_Element_Array (1 .. Length);
         Last : Stream_Element_Offset;
      begin
         pragma Assert (Stream_Element_Offset (Offset) <= Data'Last);
         SIO.Read (Input, Data, Last);
         SIO.Close (Input);
         pragma Assert (Last = Data'Last);
         Data (Stream_Element_Offset (Offset)) :=
           Data (Stream_Element_Offset (Offset)) xor 1;
         SIO.Create (Output, SIO.Out_File, Path);
         SIO.Write (Output, Data);
         SIO.Close (Output);
      end;
   end Mutate_Byte;

   procedure Truncate_Last_Byte (Source : String; Target : String) is
      package SIO renames Ada.Streams.Stream_IO;
      Input  : SIO.File_Type;
      Output : SIO.File_Type;
   begin
      SIO.Open (Input, SIO.In_File, Source);
      declare
         Length : constant Stream_Element_Count :=
           Stream_Element_Count (SIO.Size (Input));
         Data : Stream_Element_Array (1 .. Length);
         Last : Stream_Element_Offset;
      begin
         pragma Assert (Length > 1);
         SIO.Read (Input, Data, Last);
         SIO.Close (Input);
         pragma Assert (Last = Data'Last);
         SIO.Create (Output, SIO.Out_File, Target);
         SIO.Write (Output, Data (Data'First .. Data'Last - 1));
         SIO.Close (Output);
      end;
   end Truncate_Last_Byte;

   procedure Reorder_Two_Lines (Source : String; Target : String) is
      package SIO renames Ada.Streams.Stream_IO;
      Input      : SIO.File_Type;
      Output     : SIO.File_Type;
      First_End  : Stream_Element_Offset := 0;
      Second_End : Stream_Element_Offset := 0;
   begin
      SIO.Open (Input, SIO.In_File, Source);
      declare
         Length : constant Stream_Element_Count :=
           Stream_Element_Count (SIO.Size (Input));
         Data : Stream_Element_Array (1 .. Length);
         Moved : Stream_Element_Array (1 .. Length);
         Last : Stream_Element_Offset;
      begin
         SIO.Read (Input, Data, Last);
         SIO.Close (Input);
         pragma Assert (Last = Data'Last);
         for Index in Data'Range loop
            if Data (Index) = Stream_Element (Character'Pos (ASCII.LF)) then
               if First_End = 0 then
                  First_End := Index;
               else
                  Second_End := Index;
                  exit;
               end if;
            end if;
         end loop;
         pragma Assert (First_End > 0 and then Second_End = Data'Last);
         Moved (1 .. Second_End - First_End) :=
           Data (First_End + 1 .. Second_End);
         Moved (Second_End - First_End + 1 .. Second_End) :=
           Data (1 .. First_End);
         SIO.Create (Output, SIO.Out_File, Target);
         SIO.Write (Output, Moved);
         SIO.Close (Output);
      end;
   end Reorder_Two_Lines;

   procedure Append_Duplicate_First_Line (Source : String; Target : String) is
      package SIO renames Ada.Streams.Stream_IO;
      Input     : SIO.File_Type;
      Output    : SIO.File_Type;
      First_End : Stream_Element_Offset := 0;
   begin
      SIO.Open (Input, SIO.In_File, Source);
      declare
         Length : constant Stream_Element_Count :=
           Stream_Element_Count (SIO.Size (Input));
         Data : Stream_Element_Array (1 .. Length);
         Last : Stream_Element_Offset;
      begin
         SIO.Read (Input, Data, Last);
         SIO.Close (Input);
         pragma Assert (Last = Data'Last);
         for Index in Data'Range loop
            if Data (Index) = Stream_Element (Character'Pos (ASCII.LF)) then
               First_End := Index;
               exit;
            end if;
         end loop;
         pragma Assert (First_End > 0);
         SIO.Create (Output, SIO.Out_File, Target);
         SIO.Write (Output, Data);
         SIO.Write (Output, Data (1 .. First_End));
         SIO.Close (Output);
      end;
   end Append_Duplicate_First_Line;

   Append_Status : Append_Result;
   Recovered     : Recovery_Result;
   First_Hash    : Digest_Hex;
   Second_Hash   : Digest_Hex;
   Source_Hash   : constant Digest_Hex := Digest ("source-v1");

begin
   Create_Path (Test_Dir);
   Delete_If_Present (Valid_Path);
   Delete_If_Present (Corrupt_Path);
   Delete_If_Present (Truncated_Path);
   Delete_If_Present (Partial_First_Path);
   Delete_If_Present (Reordered_Path);
   Delete_If_Present (Duplicate_Path);
   Delete_If_Present (Version_Path);

   Recover (Valid_Path, Recovered);
   pragma Assert (Recovered.Status = Empty);

   Append
     (Path           => Valid_Path,
      Kind           => Mission_Created,
      State          => Draft,
      Source_Digest  => Source_Hash,
      Payload_Digest => Digest ("mission-created"),
      Result         => Append_Status,
      Event_Digest   => First_Hash);
   pragma Assert (Append_Status = Committed);

   Truncate_Last_Byte (Valid_Path, Partial_First_Path);
   Recover (Partial_First_Path, Recovered);
   pragma Assert (Recovered.Status = Truncated_Tail);
   pragma Assert (Recovered.Sequence = 0);
   pragma Assert (Recovered.Head = Zero_Digest);

   Append
     (Path           => Valid_Path,
      Kind           => State_Transitioned,
      State          => Running,
      Source_Digest  => Source_Hash,
      Payload_Digest => Digest ("running"),
      Result         => Append_Status,
      Event_Digest   => Second_Hash);
   pragma Assert (Append_Status = Committed);
   pragma Assert (First_Hash /= Second_Hash);

   Recover (Valid_Path, Recovered);
   pragma Assert (Recovered.Status = Recovered_Valid);
   pragma Assert (Recovered.Sequence = 2);
   pragma Assert (Recovered.State = Running);
   pragma Assert (Recovered.Head = Second_Hash);
   pragma Assert (Recovered.First_Payload = Digest ("mission-created"));
   pragma Assert (Recovered.Last_Payload = Digest ("running"));

   Copy_File (Valid_Path, Corrupt_Path);
   Mutate_Byte (Corrupt_Path, 100);
   Recover (Corrupt_Path, Recovered);
   pragma Assert (Recovered.Status = Corrupt);

   Truncate_Last_Byte (Valid_Path, Truncated_Path);
   Recover (Truncated_Path, Recovered);
   pragma Assert (Recovered.Status = Truncated_Tail);
   pragma Assert (Recovered.Sequence = 1);
   pragma Assert (Recovered.Head = First_Hash);
   pragma Assert (Recovered.First_Payload = Digest ("mission-created"));
   pragma Assert (Recovered.Last_Payload = Digest ("mission-created"));

   Reorder_Two_Lines (Valid_Path, Reordered_Path);
   Recover (Reordered_Path, Recovered);
   pragma Assert (Recovered.Status = Corrupt);

   Append_Duplicate_First_Line (Valid_Path, Duplicate_Path);
   Recover (Duplicate_Path, Recovered);
   pragma Assert (Recovered.Status = Corrupt);

   Copy_File (Valid_Path, Version_Path);
   Mutate_Byte (Version_Path, 1);
   Recover (Version_Path, Recovered);
   pragma Assert (Recovered.Status = Unsupported_Version);

   Put_Line ("PASS_LEDGER_RECOVERY");
end Nemesis_Ledger_Tests;
