//
//  DemoExtensionParameterAddresses.h
//  DemoExtension
//
//  Created by Jussi Viiri on 26.12.2023.
//

#pragma once

#include <AudioToolbox/AUParameters.h>

#ifdef __cplusplus
namespace DemoExtensionParameterAddress {
#endif

typedef NS_ENUM(AUParameterAddress, DemoExtensionParameterAddress) {
    gain = 0
};

#ifdef __cplusplus
}
#endif
