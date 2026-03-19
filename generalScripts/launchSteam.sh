#!/bin/bash

# Fix XBOX 360 Wireless Controller for kernels >= 6.17 and SDL < 3.4.x
export SDL_GAMECONTROLLERCONFIG=$(cat <<-END
060000000d0f00009601000000000000,Steam Controller (HHD),a:b0,b:b1,x:b2,y:b3,back:b6,guide:b8,start:b7,leftstick:b9,rightstick:b10,leftshoulder:b4,rightshoulder:b5,dpup:h0.1,dpdown:h0.4,dpleft:h0.8,dpright:h0.2,leftx:a0,lefty:a1,rightx:a3,righty:a4,lefttrigger:a2,righttrigger:a5,paddle1:b13,paddle2:b12,paddle3:b15,paddle4:b14,misc2:b11,misc3:b16,misc4:b17,crc:ea35,
0300a81c5e040000a102000000010000,X360 Wireless Controller,a:b0,b:b1,back:b6,dpdown:b12,dpleft:b13,dpright:b14,dpup:b11,guide:b8,leftshoulder:b4,leftstick:b9,lefttrigger:a2,leftx:a0,lefty:a1,rightshoulder:b5,rightstick:b10,righttrigger:a5,rightx:a3,righty:a4,start:b7,x:b2,y:b3,platform:Linux,crc:1ca8,
$SDL_GAMECONTROLLERCONFIG
END
)

/usr/bin/steam
