package Nemesis.Core.Paths with SPARK_Mode => Off is
   type Path_Decision is
     (Path_Authorized,
      Path_Not_Absolute,
      Path_Traversal,
      Path_Outside_Root,
      Path_Symlink_Escape,
      Path_Not_Found,
      Path_Already_Exists,
      Path_Missing_Parent,
      Path_Special_File,
      Path_Invalid_Root,
      Path_Error);

   function Authorize_Existing
     (Canonical_Root : String; Candidate : String) return Path_Decision;

   function Authorize_Create
     (Canonical_Root : String; Candidate : String) return Path_Decision;
end Nemesis.Core.Paths;
