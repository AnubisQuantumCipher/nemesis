package body Nemesis.Kernel.Transitions with SPARK_Mode => On is

   function Allowed
     (Source : Mission_State; Target : Mission_State) return Boolean
   is
     (case Source is
         when Draft                  => Target = Contract_Compiled,
         when Contract_Compiled      => Target = Awaiting_Authorization,
         when Awaiting_Authorization => Target = Planning,
         when Planning               => Target = Running,
         when Running                =>
           Target in Waiting_Approval
                   | Recovering
                   | Verifying
                   | Blocked_With_Evidence
                   | Cancelled,
         when Waiting_Approval       => Target in Running | Cancelled,
         when Recovering             =>
           Target in Running | Blocked_With_Evidence | Cancelled,
         when Verifying              =>
           Target in Running | Complete | Blocked_With_Evidence | Cancelled,
         when Complete | Blocked_With_Evidence | Cancelled => False);

end Nemesis.Kernel.Transitions;
