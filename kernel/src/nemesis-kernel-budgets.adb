package body Nemesis.Kernel.Budgets with SPARK_Mode => On is

   function Can_Allocate
     (Available : Budget_Record; Requested : Budget_Request) return Boolean
   is
     (Requested.Cost_Microunits <= Available.Cost_Microunits
      and then Requested.Tokens <= Available.Tokens
      and then Requested.Time_Millis <= Available.Time_Millis
      and then Requested.Processes <= Available.Processes
      and then Requested.Storage_Bytes <= Available.Storage_Bytes
      and then Requested.Network_Bytes <= Available.Network_Bytes);

   procedure Consume
     (Available : in out Budget_Record;
      Requested : Budget_Request;
      Decision  : out Budget_Decision)
   is
   begin
      if not Can_Allocate (Available, Requested) then
         Decision := Budget_Refused;
         return;
      end if;

      Available.Cost_Microunits :=
        Available.Cost_Microunits - Requested.Cost_Microunits;
      Available.Tokens := Available.Tokens - Requested.Tokens;
      Available.Time_Millis := Available.Time_Millis - Requested.Time_Millis;
      Available.Processes := Available.Processes - Requested.Processes;
      Available.Storage_Bytes :=
        Available.Storage_Bytes - Requested.Storage_Bytes;
      Available.Network_Bytes :=
        Available.Network_Bytes - Requested.Network_Bytes;
      Decision := Budget_Authorized;
   end Consume;

end Nemesis.Kernel.Budgets;
