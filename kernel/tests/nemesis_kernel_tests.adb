with Ada.Text_IO;
with Nemesis.Kernel.Types;

procedure Nemesis_Kernel_Tests is
   use Ada.Text_IO;
   use Nemesis.Kernel.Types;

   function Expected_Terminal (State : Mission_State) return Boolean is
     (State in Complete | Blocked_With_Evidence | Cancelled);

begin
   for State in Mission_State loop
      pragma Assert (Is_Terminal (State) = Expected_Terminal (State));
   end loop;

   pragma Assert (Next (Sequence_Number'First) = Sequence_Number'First + 1);
   pragma Assert (Mission_Id'Length = 26);
   pragma Assert (Authority_Decision'First = Authorized);
   pragma Assert (Authority_Decision'Last = Requires_Independent_Review);

   Put_Line ("PASS_KERNEL_TYPES");
end Nemesis_Kernel_Tests;
