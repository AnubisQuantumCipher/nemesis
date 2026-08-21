package body Nemesis.Core.JSON with SPARK_Mode => Off is

   function Is_Whitespace (Item : Character) return Boolean is
     (Item in ' ' | ASCII.HT | ASCII.LF | ASCII.CR);

   procedure Parse
     (Input  : String;
      Value  : out JSON_Object;
      Status : out Parse_Status)
   is
      Cursor : Natural := Input'First;

      procedure Skip_Whitespace is
      begin
         while Cursor <= Input'Last and then Is_Whitespace (Input (Cursor)) loop
            Cursor := Cursor + 1;
         end loop;
      end Skip_Whitespace;

      procedure Read_String
        (Output : out String; Length : out Natural; Success : out Boolean)
      is
      begin
         Output := [others => ' '];
         Length := 0;
         Success := False;
         if Cursor > Input'Last or else Input (Cursor) /= '"' then
            return;
         end if;
         Cursor := Cursor + 1;
         while Cursor <= Input'Last loop
            if Input (Cursor) = '"' then
               Cursor := Cursor + 1;
               Success := True;
               return;
            elsif Input (Cursor) = '\' or else Character'Pos (Input (Cursor)) < 32
            then
               return;
            elsif Length = Output'Length then
               return;
            else
               Length := Length + 1;
               Output (Length) := Input (Cursor);
               Cursor := Cursor + 1;
            end if;
         end loop;
      end Read_String;

      Key_Buffer : String (1 .. Max_Key_Length);
      Key_Length : Natural;
      Text_Buffer : String (1 .. Max_Value_Length);
      Text_Length : Natural;
      Success : Boolean;
      Field : Field_Record;
   begin
      Value := (Count => 0, Fields => [others => <>]);
      if Input'Length = 0 then
         Status := JSON_Empty;
         return;
      elsif Input'Length > Max_Message_Bytes then
         Status := JSON_Too_Large;
         return;
      end if;

      Skip_Whitespace;
      if Cursor > Input'Last or else Input (Cursor) /= '{' then
         Status := JSON_Malformed;
         return;
      end if;
      Cursor := Cursor + 1;
      Skip_Whitespace;
      if Cursor <= Input'Last and then Input (Cursor) = '}' then
         Cursor := Cursor + 1;
         Skip_Whitespace;
         if Cursor <= Input'Last then
            Status := JSON_Malformed;
         else
            Status := JSON_OK;
         end if;
         return;
      end if;

      loop
         if Value.Count = Max_Fields then
            Status := JSON_Too_Large;
            return;
         end if;
         Read_String (Key_Buffer, Key_Length, Success);
         if not Success or else Key_Length = 0 then
            Status := JSON_Malformed;
            return;
         end if;
         for Index in 1 .. Value.Count loop
            if Value.Fields (Index).Key_Length = Key_Length
              and then Value.Fields (Index).Key (1 .. Key_Length) =
                Key_Buffer (1 .. Key_Length)
            then
               Status := JSON_Malformed;
               return;
            end if;
         end loop;
         Skip_Whitespace;
         if Cursor > Input'Last or else Input (Cursor) /= ':' then
            Status := JSON_Malformed;
            return;
         end if;
         Cursor := Cursor + 1;
         Skip_Whitespace;

         Field :=
           (Key_Length   => Key_Length,
            Key          => [others => ' '],
            Kind         => String_Kind,
            Value_Length => 0,
            Text         => [others => ' '],
            Number       => 0,
            Flag         => False);
         Field.Key (1 .. Key_Length) := Key_Buffer (1 .. Key_Length);

         if Cursor <= Input'Last and then Input (Cursor) = '"' then
            Read_String (Text_Buffer, Text_Length, Success);
            if not Success then
               Status := JSON_Malformed;
               return;
            end if;
            Field.Kind := String_Kind;
            Field.Value_Length := Text_Length;
            if Text_Length > 0 then
               Field.Text (1 .. Text_Length) := Text_Buffer (1 .. Text_Length);
            end if;
         elsif Cursor <= Input'Last and then Input (Cursor) in '0' .. '9' then
            Field.Kind := Natural_Kind;
            while Cursor <= Input'Last and then Input (Cursor) in '0' .. '9' loop
               declare
                  Digit : constant Natural :=
                    Character'Pos (Input (Cursor)) - Character'Pos ('0');
               begin
                  if Field.Number > (Natural'Last - Digit) / 10 then
                     Status := JSON_Too_Large;
                     return;
                  end if;
                  Field.Number := Field.Number * 10 + Digit;
               end;
               Cursor := Cursor + 1;
            end loop;
         elsif Cursor + 3 <= Input'Last
           and then Input (Cursor .. Cursor + 3) = "true"
         then
            Field.Kind := Boolean_Kind;
            Field.Flag := True;
            Cursor := Cursor + 4;
         elsif Cursor + 4 <= Input'Last
           and then Input (Cursor .. Cursor + 4) = "false"
         then
            Field.Kind := Boolean_Kind;
            Field.Flag := False;
            Cursor := Cursor + 5;
         else
            Status := JSON_Unsupported_Value;
            return;
         end if;

         Value.Count := Value.Count + 1;
         Value.Fields (Value.Count) := Field;
         Skip_Whitespace;
         if Cursor > Input'Last then
            Status := JSON_Malformed;
            return;
         elsif Input (Cursor) = '}' then
            Cursor := Cursor + 1;
            exit;
         elsif Input (Cursor) = ',' then
            Cursor := Cursor + 1;
            Skip_Whitespace;
         else
            Status := JSON_Malformed;
            return;
         end if;
      end loop;

      Skip_Whitespace;
      if Cursor <= Input'Last then
         Status := JSON_Malformed;
      else
         Status := JSON_OK;
      end if;
   end Parse;

   function Find (Value : JSON_Object; Key : String) return Natural is
   begin
      if Key'Length = 0 or else Key'Length > Max_Key_Length then
         return 0;
      end if;
      for Index in 1 .. Value.Count loop
         if Value.Fields (Index).Key_Length = Key'Length
           and then Value.Fields (Index).Key (1 .. Key'Length) = Key
         then
            return Index;
         end if;
      end loop;
      return 0;
   end Find;

   function Contains (Value : JSON_Object; Key : String) return Boolean is
     (Find (Value, Key) /= 0);

   function Field_Count (Value : JSON_Object) return Natural is
     (Value.Count);

   function String_Value (Value : JSON_Object; Key : String) return String is
      Index : constant Natural := Find (Value, Key);
   begin
      if Index = 0 or else Value.Fields (Index).Kind /= String_Kind then
         raise Constraint_Error with "missing or non-string JSON field";
      end if;
      return Value.Fields (Index).Text (1 .. Value.Fields (Index).Value_Length);
   end String_Value;

   function Natural_Value (Value : JSON_Object; Key : String) return Natural is
      Index : constant Natural := Find (Value, Key);
   begin
      if Index = 0 or else Value.Fields (Index).Kind /= Natural_Kind then
         raise Constraint_Error with "missing or non-natural JSON field";
      end if;
      return Value.Fields (Index).Number;
   end Natural_Value;

   function Boolean_Value (Value : JSON_Object; Key : String) return Boolean is
      Index : constant Natural := Find (Value, Key);
   begin
      if Index = 0 or else Value.Fields (Index).Kind /= Boolean_Kind then
         raise Constraint_Error with "missing or non-boolean JSON field";
      end if;
      return Value.Fields (Index).Flag;
   end Boolean_Value;

end Nemesis.Core.JSON;
