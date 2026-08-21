with Ada.Strings.Unbounded;
with Nemesis.Core.Ledger;

package Nemesis.Core.Objects with SPARK_Mode => Off is
   use Nemesis.Core.Ledger;

   type Store_Result is
     (Stored,
      Already_Present,
      Collision_Detected,
      Store_Write_Failed,
      Store_Sync_Failed,
      Store_Close_Failed,
      Store_Rename_Failed);

   type Load_Result is
     (Loaded_Valid, Object_Not_Found, Integrity_Failed, Load_Failed);

   function Path_Of (Root : String; Object_Digest : Digest_Hex) return String;

   procedure Put
     (Root          : String;
      Data          : String;
      Result        : out Store_Result;
      Object_Digest : out Digest_Hex);

   procedure Get
     (Root          : String;
      Object_Digest : Digest_Hex;
      Result        : out Load_Result;
      Data          : out Ada.Strings.Unbounded.Unbounded_String);
end Nemesis.Core.Objects;
