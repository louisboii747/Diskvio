import XCTest

@MainActor
final class DiskvioUITests: XCTestCase {
    override func setUpWithError() throws {
        continueAfterFailure = false
    }

    func testWorkspaceInBothAppearances() throws {
        for appearance in ["Light", "Dark"] {
            let app = XCUIApplication()
            app.launchArguments = ["-AppleInterfaceStyle", appearance, "-ApplePersistenceIgnoreState", "YES"]
            app.launch()
            let search = app.textFields["Search devices and volumes"]
            XCTAssertTrue(search.waitForExistence(timeout: 10))
            let refresh = app.buttons["Refresh"].firstMatch
            let ready = NSPredicate(format: "enabled == true")
            expectation(for: ready, evaluatedWith: refresh)
            waitForExpectations(timeout: 30)
            let attachment = XCTAttachment(screenshot: app.screenshot())
            attachment.name = "macos-\(appearance.lowercased())-workspace"
            attachment.lifetime = .keepAlways
            add(attachment)
            search.click()
            search.typeText("diskvio-search-no-match-314159")
            XCTAssertEqual(search.value as? String, "diskvio-search-no-match-314159")
            let clear = app.buttons["Clear search"]
            XCTAssertTrue(clear.exists)
            clear.click()
            XCTAssertEqual(search.value as? String, "")
            XCTAssertTrue(app.buttons["Inspector"].firstMatch.exists)
            app.terminate()
        }
    }
}
