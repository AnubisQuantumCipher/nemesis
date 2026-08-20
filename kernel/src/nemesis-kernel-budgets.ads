package Nemesis.Kernel.Budgets with SPARK_Mode => On is
   type Budget_Unit is range 0 .. 2 ** 63 - 1;

   type Budget_Record is record
      Cost_Microunits : Budget_Unit;
      Tokens          : Budget_Unit;
      Time_Millis     : Budget_Unit;
      Processes       : Budget_Unit;
      Storage_Bytes   : Budget_Unit;
      Network_Bytes   : Budget_Unit;
   end record;

   subtype Budget_Request is Budget_Record;
   type Budget_Decision is (Budget_Authorized, Budget_Refused);

   function Can_Allocate
     (Available : Budget_Record; Requested : Budget_Request) return Boolean
   with Global => null;

   procedure Consume
     (Available : in out Budget_Record;
      Requested : Budget_Request;
      Decision  : out Budget_Decision)
   with
     Global => null,
     Post =>
       (if Decision = Budget_Refused then Available = Available'Old
        else
          Available.Cost_Microunits =
            Available'Old.Cost_Microunits - Requested.Cost_Microunits
          and then Available.Tokens =
            Available'Old.Tokens - Requested.Tokens
          and then Available.Time_Millis =
            Available'Old.Time_Millis - Requested.Time_Millis
          and then Available.Processes =
            Available'Old.Processes - Requested.Processes
          and then Available.Storage_Bytes =
            Available'Old.Storage_Bytes - Requested.Storage_Bytes
          and then Available.Network_Bytes =
            Available'Old.Network_Bytes - Requested.Network_Bytes);
end Nemesis.Kernel.Budgets;
