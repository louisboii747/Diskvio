import Foundation

nonisolated enum WorkspaceLayout {
    static let windowMinimumWidth: CGFloat = 1040
    static let sidebarMinimum: CGFloat = 260
    static let sidebarIdeal: CGFloat = 320
    static let sidebarMaximum: CGFloat = 480
    static let contentMinimum: CGFloat = 480
    static let inspectorMinimum: CGFloat = 240
    static let inspectorIdeal: CGFloat = 280
    static let inspectorMaximum: CGFloat = 340

    static func allowsInlineInspector(width: CGFloat) -> Bool {
        width >= contentMinimum + inspectorIdeal + 40
    }
}
