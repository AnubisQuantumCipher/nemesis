// Prints the number of on-screen CoreGraphics windows owned by the given pid,
// followed by the bounds of each. Works under a locked console (window-server
// enumeration does not require an unlocked GUI session). Exit 0 with count>=1
// means the process created at least one real window.
import CoreGraphics
import Foundation

guard CommandLine.arguments.count == 2, let pid = Int32(CommandLine.arguments[1]) else {
    FileHandle.standardError.write("usage: probe_window_ownership.swift <pid>\n".data(using: .utf8)!)
    exit(2)
}
let options: CGWindowListOption = [.optionOnScreenOnly]
guard let info = CGWindowListCopyWindowInfo(options, kCGNullWindowID) as? [[String: Any]] else {
    print("WINDOWS 0")
    exit(1)
}
let owned = info.filter { ($0[kCGWindowOwnerPID as String] as? Int32) == pid }
print("WINDOWS \(owned.count)")
for window in owned {
    if let bounds = window[kCGWindowBounds as String] as? [String: Any] {
        let w = bounds["Width"] ?? "?", h = bounds["Height"] ?? "?"
        print("BOUNDS \(w)x\(h)")
    }
}
exit(owned.isEmpty ? 1 : 0)
