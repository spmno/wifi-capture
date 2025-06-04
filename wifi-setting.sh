sudo ifconfig wlx00e04bd3ded6 down
sleep 1
sudo iwconfig wlx00e04bd3ded6 mode monitor
sleep 1
sudo ifconfig wlx00e04bd3ded6 up
sleep 1
sudo iwconfig wlx00e04bd3ded6 channel 6