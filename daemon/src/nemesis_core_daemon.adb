with Ada.Command_Line;
with Ada.Directories;
with Ada.Streams;
with Ada.Strings;
with Ada.Strings.Fixed;
with Ada.Strings.Unbounded;
with GNAT.Sockets;
with Interfaces.C;
with Interfaces.C.Strings;
with Nemesis.Core.JSON;
with Nemesis.Core.Ledger;
with Nemesis.Core.Mission_Store;
with Nemesis.Kernel.Capabilities;
with Nemesis.Kernel.Completion;
with Nemesis.Kernel.Missions;
with Nemesis.Kernel.Types;

procedure Nemesis_Core_Daemon is
   use Ada.Strings.Unbounded;
   use GNAT.Sockets;
   use Nemesis.Core.JSON;
   use Nemesis.Core.Ledger;
   use Nemesis.Core.Mission_Store;
   use Nemesis.Kernel.Capabilities;
   use Nemesis.Kernel.Completion;
   use Nemesis.Kernel.Missions;
   use Nemesis.Kernel.Types;
   use type Ada.Streams.Stream_Element_Offset;
   use type Interfaces.C.int;

   Quote : constant Character := Character'Val (34);
   Zero : constant Digest_256 := [others => '0'];

   function C_Chmod
     (Path : Interfaces.C.Strings.chars_ptr;
      Mode : Interfaces.C.unsigned) return Interfaces.C.int
   with Import, Convention => C, External_Name => "chmod";

   function C_Unlink
     (Path : Interfaces.C.Strings.chars_ptr) return Interfaces.C.int
   with Import, Convention => C, External_Name => "unlink";

   function Number_Image (Value : Sequence_Number) return String is
     (Ada.Strings.Fixed.Trim (Sequence_Number'Image (Value), Ada.Strings.Both));


   function State_Image (Value : Mission_State) return String is
     (Mission_State'Image (Value));

   function Status_Response (Status : String) return String is
     ("{" & Quote & "status" & Quote & ":" & Quote & Status & Quote & "}");

   function Context_Response
     (Status : String; Context : Mission_Context; Extra : String := "")
      return String
   is
     ("{" & Quote & "status" & Quote & ":" & Quote & Status & Quote & ","
      & Quote & "state" & Quote & ":" & Quote
      & State_Image (State_Of (Context.Mission)) & Quote & ","
      & Quote & "sequence" & Quote & ":"
      & Number_Image (Sequence_Of (Context.Mission)) & ","
      & Quote & "ledger_head" & Quote & ":" & Quote
      & String (Context.Ledger_Head) & Quote & Extra & "}");

   function Valid_Id (Value : String; Prefix : String) return Boolean is
     (Value'Length = 26
      and then Value (Value'First .. Value'First + Prefix'Length - 1) = Prefix
      and then
        (for all Index in Value'First + Prefix'Length .. Value'Last =>
           Value (Index) in 'a' .. 'z' | '0' .. '9'));

   function Valid_Digest (Value : String) return Boolean is
     (Value'Length = 64
      and then
        (for all Item of Value => Item in 'a' .. 'f' | '0' .. '9'));

   function Payload_Digest (Value : String) return Digest_256 is
     (Digest_256 (Nemesis.Core.Ledger.Digest (Value)));

   function Exact_Fields (Value : JSON_Object; Count : Natural) return Boolean is
     (Field_Count (Value) = Count);

   function Process_Request (Home : String; Input : String) return String is
      Request : JSON_Object;
      JSON_Status : Parse_Status;
      Command : Unbounded_String;
      Context : Mission_Context;
      Store_Result : Store_Status;
   begin
      Parse (Input, Request, JSON_Status);
      if JSON_Status /= JSON_OK
        or else not Contains (Request, "schema")
        or else String_Value (Request, "schema") /= "nemesis.local/v1"
        or else not Contains (Request, "command")
      then
         return Status_Response ("INVALID_REQUEST");
      end if;
      Command := To_Unbounded_String (String_Value (Request, "command"));

      if Command = "ping" then
         if not Exact_Fields (Request, 2) then
            return Status_Response ("INVALID_REQUEST");
         end if;
         return Status_Response ("OK");
      elsif Command = "create" then
         if not Exact_Fields (Request, 7) then
            return Status_Response ("INVALID_REQUEST");
         end if;
         declare
            Id_Text : constant String := String_Value (Request, "mission_id");
            Worker_Text : constant String := String_Value (Request, "worker_id");
            Contract_Text : constant String :=
              String_Value (Request, "contract_digest");
            Scope_Text : constant String := String_Value (Request, "scope_digest");
            Source_Text : constant String := String_Value (Request, "source_digest");
         begin
            if not Valid_Id (Id_Text, "mis_")
              or else not Valid_Id (Worker_Text, "wrk_")
              or else not Valid_Digest (Contract_Text)
              or else not Valid_Digest (Scope_Text)
              or else not Valid_Digest (Source_Text)
            then
               return Status_Response ("INVALID_REQUEST");
            end if;
            Create_Mission
              (Home,
               Mission_Id (Id_Text),
               Worker_Id (Worker_Text),
               Digest_256 (Contract_Text),
               Digest_256 (Scope_Text),
               Digest_256 (Source_Text),
               Store_Result,
               Context);
            if Store_Result = Store_OK then
               return Context_Response ("OK", Context);
            else
               return Status_Response (Store_Status'Image (Store_Result));
            end if;
         end;
      end if;

      if not Contains (Request, "mission_id") then
         return Status_Response ("INVALID_REQUEST");
      end if;
      declare
         Id_Text : constant String := String_Value (Request, "mission_id");
      begin
         if not Valid_Id (Id_Text, "mis_") then
            return Status_Response ("INVALID_REQUEST");
         end if;
         Load_Mission (Home, Mission_Id (Id_Text), Store_Result, Context);
      end;
      if Store_Result /= Store_OK then
         return Status_Response (Store_Status'Image (Store_Result));
      end if;

      if Command = "inspect" then
         if not Exact_Fields (Request, 3) then
            return Status_Response ("INVALID_REQUEST");
         end if;
         return Context_Response ("OK", Context);
      elsif Command = "authorize" then
         if not Exact_Fields (Request, 4) then
            return Status_Response ("INVALID_REQUEST");
         end if;
         declare
            Contract_Text : constant String :=
              String_Value (Request, "contract_digest");
         begin
            if not Valid_Digest (Contract_Text)
              or else Digest_256 (Contract_Text) /= Context.Contract_Digest
            then
               return Context_Response ("REFUSED", Context);
            end if;
         end;
         declare
            Authorized_Contract : constant Digest_256 := Context.Contract_Digest;
         begin
            Commit_Transition
              (Home, Context, Planning, Contract_Authorized,
               Authorized_Contract, Store_Result);
         end;
         if Store_Result = Store_OK then
            return Context_Response ("OK", Context);
         else
            return Context_Response ("REFUSED", Context);
         end if;
      elsif Command = "run" then
         if not Exact_Fields (Request, 3) then
            return Status_Response ("INVALID_REQUEST");
         end if;
         Commit_Transition
           (Home, Context, Running, State_Transitioned,
            Payload_Digest ("RUNNING"), Store_Result);
         if Store_Result = Store_OK then
            return Context_Response ("OK", Context);
         else
            return Context_Response ("REFUSED", Context);
         end if;
      elsif Command = "authorize_action" then
         if not Exact_Fields (Request, 7)
           or else State_Of (Context.Mission) /= Running
         then
            return Context_Response ("REFUSED", Context);
         end if;
         declare
            Worker_Text : constant String := String_Value (Request, "worker_id");
            Scope_Text : constant String := String_Value (Request, "scope_digest");
            Action_Text : constant String := String_Value (Request, "action_digest");
            Estimated : constant Natural := Natural_Value (Request, "estimated_bytes");
            Current : constant Sequence_Number := Sequence_Of (Context.Mission);
            Expiry : constant Sequence_Number :=
              (if Current > Sequence_Number'Last - 100
               then Sequence_Number'Last else Current + 100);
            Grant : Capability_Grant;
            Action : Action_Request;
            Decision : Authority_Decision;
         begin
            if not Valid_Id (Worker_Text, "wrk_")
              or else not Valid_Digest (Scope_Text)
              or else not Valid_Digest (Action_Text)
            then
               return Status_Response ("INVALID_REQUEST");
            end if;
            Grant :=
              (Id            => "cap_0000000000000000000000",
               Mission       => Context.Id,
               Subject       => Context.Worker,
               Resource      => Filesystem,
               Operations    => [Modify_Data => True, others => False],
               Scope         => Context.Scope_Digest,
               Expires_After => Expiry,
               Maximum_Bytes => 4_096,
               Status        => Active);
            Action :=
              (Mission         => Context.Id,
               Subject         => Worker_Id (Worker_Text),
               Resource        => Filesystem,
               Operation       => Modify_Data,
               Scope           => Digest_256 (Scope_Text),
               Estimated_Bytes => Estimated,
               Digest          => Digest_256 (Action_Text));
            Decision := Authorize (Grant, Action, Current);
            if Decision /= Authorized then
               return Context_Response
                 ("REFUSED", Context,
                  "," & Quote & "decision" & Quote & ":" & Quote
                  & Authority_Decision'Image (Decision) & Quote);
            end if;
            Commit_Event
              (Home, Context, Action_Authorized, Digest_256 (Action_Text),
               Store_Result);
            if Store_Result = Store_OK then
               return Context_Response
                 ("OK", Context,
                  "," & Quote & "decision" & Quote & ":" & Quote
                  & "AUTHORIZED" & Quote);
            else
               return Context_Response ("REFUSED", Context);
            end if;
         end;
      elsif Command = "action_completed" then
         if not Exact_Fields (Request, 5)
           or else State_Of (Context.Mission) /= Running
         then
            return Context_Response ("REFUSED", Context);
         end if;
         declare
            Source_Text : constant String := String_Value (Request, "source_digest");
            Action_Text : constant String := String_Value (Request, "action_digest");
         begin
            if not Valid_Digest (Source_Text) or else not Valid_Digest (Action_Text)
            then
               return Status_Response ("INVALID_REQUEST");
            end if;
            Record_Source
              (Home, Context, Digest_256 (Source_Text), Digest_256 (Action_Text),
               Store_Result);
         end;
         if Store_Result = Store_OK then
            return Context_Response ("OK", Context);
         else
            return Context_Response ("REFUSED", Context);
         end if;
      elsif Command = "accept_evidence" then
         if not Exact_Fields (Request, 6)
           or else State_Of (Context.Mission) /= Running
         then
            return Context_Response ("REFUSED", Context);
         end if;
         declare
            Claim : constant String := String_Value (Request, "claim_id");
            Source_Text : constant String := String_Value (Request, "source_digest");
            Evidence_Text : constant String := String_Value (Request, "evidence_digest");
         begin
            if not Valid_Digest (Source_Text) or else not Valid_Digest (Evidence_Text)
            then
               return Status_Response ("INVALID_REQUEST");
            end if;
            Record_Evidence
              (Home, Context, Claim, Digest_256 (Source_Text),
               Digest_256 (Evidence_Text), Store_Result);
         end;
         if Store_Result = Store_OK then
            return Context_Response ("OK", Context);
         elsif Store_Result = Evidence_Stale then
            return Context_Response ("EVIDENCE_STALE", Context);
         else
            return Context_Response ("REFUSED", Context);
         end if;
      elsif Command = "propose_completion" then
         if not Exact_Fields (Request, 3)
           or else State_Of (Context.Mission) /= Running
         then
            return Context_Response ("REFUSED", Context);
         end if;
         Commit_Transition
           (Home, Context, Verifying, Completion_Proposed,
            Payload_Digest ("COMPLETION_PROPOSED"), Store_Result);
         if Store_Result /= Store_OK then
            return Context_Response ("REFUSED", Context);
         end if;
         declare
            Claims : constant Claim_Status_Array (1 .. 2) :=
              [1 =>
                 (if Context.Build_Evidence /= Zero
                    and then Context.Build_Source = Context.Current_Source
                  then Supported_Current else Required_Missing),
               2 =>
                 (if Context.Test_Evidence /= Zero
                    and then Context.Test_Source = Context.Current_Source
                  then Supported_Current else Required_Missing)];
            Decision : constant Completion_Decision :=
              Evaluate (Claims, Has_Blocker => False, Worker_Proposed => True);
         begin
            if Decision = Completion_Accepted then
               declare
                  Final_Source : constant Digest_256 := Context.Current_Source;
               begin
                  Commit_Transition
                    (Home, Context, Complete, Mission_Completed,
                     Final_Source, Store_Result);
               end;
               if Store_Result = Store_OK then
                  return Context_Response ("OK", Context);
               end if;
            elsif Decision = Completion_Returned_To_Running then
               Commit_Transition
                 (Home, Context, Running, State_Transitioned,
                  Payload_Digest ("EVIDENCE_STALE"), Store_Result);
               return Context_Response ("EVIDENCE_STALE", Context);
            end if;
            return Context_Response ("BLOCKED_WITH_EVIDENCE", Context);
         end;
      elsif Command = "receipt_payload" then
         if not Exact_Fields (Request, 3)
           or else State_Of (Context.Mission) /= Complete
           or else not Claims_Current (Context)
         then
            return Context_Response ("REFUSED", Context);
         end if;
         return
           "{" & Quote & "status" & Quote & ":" & Quote & "OK" & Quote & ","
           & Quote & "mission_id" & Quote & ":" & Quote & String (Context.Id)
           & Quote & "," & Quote & "event_sequence" & Quote & ":"
           & Number_Image (Sequence_Of (Context.Mission)) & ","
           & Quote & "terminal_state" & Quote & ":" & Quote & "COMPLETE"
           & Quote & "," & Quote & "source_digest" & Quote & ":" & Quote
           & String (Context.Current_Source) & Quote & ","
           & Quote & "ledger_head" & Quote & ":" & Quote
           & String (Context.Ledger_Head) & Quote & ","
           & Quote & "build_evidence" & Quote & ":" & Quote
           & String (Context.Build_Evidence) & Quote & ","
           & Quote & "test_evidence" & Quote & ":" & Quote
           & String (Context.Test_Evidence) & Quote & ","
           & Quote & "claims_verified" & Quote & ":true}";
      else
         return Status_Response ("UNKNOWN_COMMAND");
      end if;
   exception
      when others =>
         return Status_Response ("INVALID_REQUEST");
   end Process_Request;

   procedure Send_Line (Socket : Socket_Type; Line : String) is
      Data : Ada.Streams.Stream_Element_Array
        (1 .. Ada.Streams.Stream_Element_Offset (Line'Length + 1));
      Last : Ada.Streams.Stream_Element_Offset;
   begin
      for Index in Line'Range loop
         Data
           (Ada.Streams.Stream_Element_Offset (Index - Line'First + 1)) :=
             Ada.Streams.Stream_Element (Character'Pos (Line (Index)));
      end loop;
      Data (Data'Last) := Ada.Streams.Stream_Element (Character'Pos (ASCII.LF));
      Send_Socket (Socket, Data, Last);
      if Last /= Data'Last then
         raise Socket_Error with "partial response write";
      end if;
   end Send_Line;

   function Receive_Line (Socket : Socket_Type; Valid : out Boolean) return String is
      Data : Ada.Streams.Stream_Element_Array (1 .. 4_096);
      Last : Ada.Streams.Stream_Element_Offset;
      Message : Unbounded_String;
   begin
      Valid := False;
      loop
         Receive_Socket (Socket, Data, Last);
         if Last < Data'First then
            return To_String (Message);
         end if;
         for Index in Data'First .. Last loop
            if Character'Val (Data (Index)) = ASCII.LF then
               Valid := Length (Message) > 0;
               return To_String (Message);
            end if;
            if Length (Message) = Max_Message_Bytes then
               return To_String (Message);
            end if;
            Append (Message, Character'Val (Data (Index)));
         end loop;
      end loop;
   end Receive_Line;

   Home : Unbounded_String;
   Socket_Path : Unbounded_String;
   Server : Socket_Type;
   Client : Socket_Type;
   Address : Sock_Addr_Type;

begin
   if Ada.Command_Line.Argument_Count /= 2
     or else Ada.Command_Line.Argument (1) /= "--home"
   then
      Ada.Command_Line.Set_Exit_Status (Ada.Command_Line.Failure);
      return;
   end if;
   Home := To_Unbounded_String (Ada.Command_Line.Argument (2));
   Ada.Directories.Create_Path (To_String (Home));
   declare
      Path : Interfaces.C.Strings.chars_ptr :=
        Interfaces.C.Strings.New_String (To_String (Home));
      Code : Interfaces.C.int;
   begin
      Code := C_Chmod (Path, 16#1C0#);
      Interfaces.C.Strings.Free (Path);
      if Code /= 0 then
         raise Socket_Error with "failed to restrict local home permissions";
      end if;
   end;
   Socket_Path := To_Unbounded_String (To_String (Home) & "/core.sock");
   declare
      Path : Interfaces.C.Strings.chars_ptr :=
        Interfaces.C.Strings.New_String (To_String (Socket_Path));
   begin
      if C_Unlink (Path) /= 0 then
         null;
      end if;
      Interfaces.C.Strings.Free (Path);
   end;

   Create_Socket (Server, Family_Unix, Socket_Stream);
   Bind_Socket (Server, Unix_Socket_Address (To_String (Socket_Path)));
   declare
      Path : Interfaces.C.Strings.chars_ptr :=
        Interfaces.C.Strings.New_String (To_String (Socket_Path));
      Code : Interfaces.C.int;
   begin
      Code := C_Chmod (Path, 16#180#);
      Interfaces.C.Strings.Free (Path);
      if Code /= 0 then
         raise Socket_Error with "failed to restrict local socket permissions";
      end if;
   end;
   Listen_Socket (Server, 16);

   loop
      Accept_Socket (Server, Client, Address);
      declare
         Valid : Boolean;
         Request : constant String := Receive_Line (Client, Valid);
         Response : constant String :=
           (if Valid then Process_Request (To_String (Home), Request)
            else Status_Response ("INVALID_REQUEST"));
      begin
         Send_Line (Client, Response);
      exception
         when others =>
            null;
      end;
      Close_Socket (Client);
   end loop;
exception
   when others =>
      begin
         Close_Socket (Client);
      exception
         when others =>
            null;
      end;
      begin
         Close_Socket (Server);
      exception
         when others =>
            null;
      end;
      Ada.Command_Line.Set_Exit_Status (Ada.Command_Line.Failure);
end Nemesis_Core_Daemon;
