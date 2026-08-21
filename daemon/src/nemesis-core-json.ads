package Nemesis.Core.JSON with SPARK_Mode => Off is
   Max_Message_Bytes : constant Positive := 65_536;

   type Parse_Status is
     (JSON_OK, JSON_Empty, JSON_Too_Large, JSON_Malformed, JSON_Unsupported_Value);

   type JSON_Object is private;

   procedure Parse
     (Input  : String;
      Value  : out JSON_Object;
      Status : out Parse_Status);

   function Contains (Value : JSON_Object; Key : String) return Boolean;
   function Field_Count (Value : JSON_Object) return Natural;
   function String_Value (Value : JSON_Object; Key : String) return String;
   function Natural_Value (Value : JSON_Object; Key : String) return Natural;
   function Boolean_Value (Value : JSON_Object; Key : String) return Boolean;

private
   Max_Fields : constant Positive := 16;
   Max_Key_Length : constant Positive := 32;
   Max_Value_Length : constant Positive := 4_096;

   type Value_Kind is (String_Kind, Natural_Kind, Boolean_Kind);
   type Field_Record is record
      Key_Length   : Natural range 0 .. Max_Key_Length := 0;
      Key          : String (1 .. Max_Key_Length) := [others => ' '];
      Kind         : Value_Kind := String_Kind;
      Value_Length : Natural range 0 .. Max_Value_Length := 0;
      Text         : String (1 .. Max_Value_Length) := [others => ' '];
      Number       : Natural := 0;
      Flag         : Boolean := False;
   end record;
   type Field_Array is array (Positive range 1 .. Max_Fields) of Field_Record;
   type JSON_Object is record
      Count  : Natural range 0 .. Max_Fields := 0;
      Fields : Field_Array;
   end record;
end Nemesis.Core.JSON;
