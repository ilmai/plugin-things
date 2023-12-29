//
//  DemoExtensionAudioUnit.h
//  DemoExtension
//
//  Created by Jussi Viiri on 26.12.2023.
//

#import <AudioToolbox/AudioToolbox.h>
#import <AVFoundation/AVFoundation.h>

@interface DemoExtensionAudioUnit : AUAudioUnit
- (void)setupParameterTree:(AUParameterTree *)parameterTree;
@end
