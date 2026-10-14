import Foundation

nonisolated enum WorkspaceLayout {
    static let windowMinimumWidth: CGFloat = 900
    static let sidebarMinimum: CGFloat = 240
    static let sidebarIdeal: CGFloat = 280
    static let sidebarMaximum: CGFloat = 400
    static let contentMinimum: CGFloat = 440
    static let inspectorMinimum: CGFloat = 240
    static let inspectorIdeal: CGFloat = 280
    static let inspectorMaximum: CGFloat = 340

    static func allowsInlineInspector(width: CGFloat) -> Bool {
        width >= contentMinimum + inspectorIdeal + 40
    }
}
