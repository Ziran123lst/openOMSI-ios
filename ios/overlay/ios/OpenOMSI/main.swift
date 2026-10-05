// openOMSI on iOS: the process's entry point.
//
// Everything the player sees is drawn by the Rust game (`crates/omsi-app`, linked as
// `libopenomsi_game.a`): winit's iOS backend calls `UIApplicationMain` itself inside
// `EventLoop::run_app`, so this shell only computes the sandbox folders and hands them to
// `openomsi_ios_start` (crates/omsi-app/src/ios.rs) - which never returns, because the
// Rust event loop runs until the process ends.
//
// The content folder (`Documents/openOMSI`) is where the player imports the OMSI 2 folder
// and the mods through the Files app (`UIFileSharingEnabled` makes it reachable there;
// see docs/IOS.md).

import Foundation
import UIKit

// The C entry point of the Rust library.
@_silgen_name("openomsi_ios_start")
func openomsi_ios_start(_ home: UnsafePointer<CChar>, _ content: UnsafePointer<CChar>)

// This top-level code runs on the main thread, before UIApplicationMain - exactly where
// winit's iOS backend wants the event loop to be created.
let home = NSHomeDirectory()
let content = home + "/Documents/openOMSI"

// The content folder exists before the game looks at it (README.txt, env.txt, crash.log).
try? FileManager.default.createDirectory(atPath: content, withIntermediateDirectories: true)

home.withCString { h in
    content.withCString { c in
        openomsi_ios_start(h, c)
    }
}
