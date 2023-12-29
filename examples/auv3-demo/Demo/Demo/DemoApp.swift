//
//  DemoApp.swift
//  Demo
//
//  Created by Jussi Viiri on 26.12.2023.
//

import CoreMIDI
import SwiftUI

@main
struct DemoApp: App {
    @ObservedObject private var hostModel = AudioUnitHostModel()

    var body: some Scene {
        WindowGroup {
            ContentView(hostModel: hostModel)
        }
    }
}
