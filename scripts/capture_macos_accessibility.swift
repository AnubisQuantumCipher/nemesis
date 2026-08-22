#!/usr/bin/env swift

import ApplicationServices
import Foundation

struct AccessibilityNode: Codable {
    let depth: Int
    let role: String
    let title: String?
    let description: String?
    let value: String?
    let help: String?
    let enabled: Bool?
    let focused: Bool?
}

struct AccessibilityCapture: Codable {
    let schema: String
    let pid: Int32
    let capturedAt: String
    let nodes: [AccessibilityNode]
}

func attribute(_ element: AXUIElement, _ name: CFString) -> CFTypeRef? {
    var value: CFTypeRef?
    guard AXUIElementCopyAttributeValue(element, name, &value) == .success else {
        return nil
    }
    return value
}

func stringAttribute(_ element: AXUIElement, _ name: CFString) -> String? {
    guard let value = attribute(element, name) else { return nil }
    if let string = value as? String { return string }
    if let number = value as? NSNumber { return number.stringValue }
    return String(describing: value)
}

func boolAttribute(_ element: AXUIElement, _ name: CFString) -> Bool? {
    guard let value = attribute(element, name) else { return nil }
    return (value as? NSNumber)?.boolValue
}

func childElements(_ element: AXUIElement) -> [AXUIElement] {
    guard let value = attribute(element, kAXChildrenAttribute as CFString) else { return [] }
    return value as? [AXUIElement] ?? []
}

func walk(
    _ element: AXUIElement,
    depth: Int,
    nodes: inout [AccessibilityNode],
    maximumNodes: Int
) {
    guard depth <= 24, nodes.count < maximumNodes else { return }
    nodes.append(
        AccessibilityNode(
            depth: depth,
            role: stringAttribute(element, kAXRoleAttribute as CFString) ?? "UNKNOWN",
            title: stringAttribute(element, kAXTitleAttribute as CFString),
            description: stringAttribute(element, kAXDescriptionAttribute as CFString),
            value: stringAttribute(element, kAXValueAttribute as CFString),
            help: stringAttribute(element, kAXHelpAttribute as CFString),
            enabled: boolAttribute(element, kAXEnabledAttribute as CFString),
            focused: boolAttribute(element, kAXFocusedAttribute as CFString)
        )
    )
    for child in childElements(element) {
        walk(child, depth: depth + 1, nodes: &nodes, maximumNodes: maximumNodes)
        if nodes.count >= maximumNodes { return }
    }
}

func fail(_ message: String) -> Never {
    FileHandle.standardError.write(Data("FAIL_NATIVE_ACCESSIBILITY \(message)\n".utf8))
    exit(1)
}

guard CommandLine.arguments.count == 3,
      let pid = Int32(CommandLine.arguments[1]) else {
    fail("usage: capture_macos_accessibility.swift <pid> <output.json>")
}
let output = URL(fileURLWithPath: CommandLine.arguments[2])
if let session = CGSessionCopyCurrentDictionary() as? [String: Any],
   session["CGSSessionScreenIsLocked"] as? Bool == true {
    fail("session_locked")
}
let application = AXUIElementCreateApplication(pid)
var windowsValue: CFTypeRef?
guard AXUIElementCopyAttributeValue(
    application,
    kAXWindowsAttribute as CFString,
    &windowsValue
) == .success,
      let windows = windowsValue as? [AXUIElement],
      !windows.isEmpty else {
    fail("no_accessible_window pid=\(pid)")
}
var nodes: [AccessibilityNode] = []
for window in windows {
    walk(window, depth: 0, nodes: &nodes, maximumNodes: 4096)
}
if nodes.isEmpty {
    fail("empty_tree pid=\(pid)")
}
guard nodes.contains(where: { $0.role == "AXWindow" }),
      nodes.contains(where: {
          ["AXWebArea", "AXButton", "AXTextField", "AXStaticText"].contains($0.role)
      }) else {
    fail("tree_missing_window_or_content_roles")
}
let formatter = ISO8601DateFormatter()
formatter.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
let capture = AccessibilityCapture(
    schema: "nemesis.macos-accessibility/v1",
    pid: pid,
    capturedAt: formatter.string(from: Date()),
    nodes: nodes
)
let encoder = JSONEncoder()
encoder.outputFormatting = [.prettyPrinted, .sortedKeys, .withoutEscapingSlashes]
var bytes = try encoder.encode(capture)
bytes.append(0x0A)
try FileManager.default.createDirectory(
    at: output.deletingLastPathComponent(),
    withIntermediateDirectories: true
)
try bytes.write(to: output, options: .atomic)
print("PASS_NATIVE_ACCESSIBILITY_TREE nodes=\(nodes.count) output=\(output.path)")
