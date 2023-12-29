//
//  DemoExtensionMainView.swift
//  DemoExtension
//
//  Created by Jussi Viiri on 26.12.2023.
//

import SwiftUI

struct DemoExtensionMainView: View {
    var parameterTree: ObservableAUParameterGroup
    
    var body: some View {
        ParameterSlider(param: parameterTree.global.gain)
    }
}
