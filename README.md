
[![Go Report](https://goreportcard.com/badge/github.com/vany/controlrake?logo=go&logoColor=white&style=flat-square]][goreport-url)](https://goreportcard.com/report/github.com/vany/controlrake)


# controlrake
Contorl deck for streaming and other activities. Web interface on small device controls server on streaming device.


Main idea - this is a tiny webserver, that displays buttons in accordance with config,
receives and sends events to and from web iface and then execute commands on server side.
Or monitors state and shows it on corresponding widget


## The show
In fact all development process is a show. 
You can attend it every monday/wensday/friday evening.
[on twitch](https://www.twitch.tv/vanyserezhkin) or [Youtube](https://www.youtube.com/@vanyserezhkin)


## How to use
You can watch entire show, but it is impossible for regular human.
1. Configure config.yml (it have working defaults, so you can reconfigure widgets when you ned it)
2. Add http://localhost:8888/static/obs.html to OBS as websource and fit it boundaries to the screen.
3. Disable screen locking on mobile device (also it is a good idea to start charging it)
4. Launch application, scan qr code with mobile phone and you're done, you can control your stream from deck aside of your laptop.

## Live ideas and progress traking
https://github.com/users/Vany/projects/1
and in
https://gist.github.com/Vany/d7263fa45e39b38cef91f63d9d7e3caa

