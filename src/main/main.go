// LICENSE APACHE 2.0
// (C) Vany Serezhkin 2020

package main

import (
	"context"
	"fmt"
	"github.com/mdp/qrterminal/v3"
	"github.com/vany/controlrake/src/app"
	"github.com/vany/controlrake/src/config"
	"github.com/vany/controlrake/src/http"
	"github.com/vany/controlrake/src/obs"
	"github.com/vany/controlrake/src/obsbrowser"
	"github.com/vany/controlrake/src/types"
	"github.com/vany/controlrake/src/widget"
	. "github.com/vany/pirog"
	"net"
	"os"
	"os/signal"
	"regexp"
	"strings"
)

func main() {

	ctx, cf := signal.NotifyContext(context.Background(), os.Interrupt)
	defer cf()
	ctx = app.PutToApp(ctx, MUST2(config.ReadConfig(ctx)))
	ctx = app.PutToApp(ctx, types.NewLogger())
	con := app.FromContext(ctx)
	ctx = app.PutToApp(ctx, obs.New(ctx))
	ctx = app.PutToApp(ctx, widget.New(ctx, con.Cfg.Widget))
	ctx = app.PutToApp(ctx, obsbrowser.New(ctx))

	GetMyAddrs(ctx)

	MUST(http.ListenAndServe(ctx))
}

var v4Re = regexp.MustCompile(`^\d+\.\d+\.\d+\.\d+`)

// TODO rethink design
func GetMyAddrs(ctx context.Context) {
	ifs, err := net.Interfaces()
	if err != nil {
		panic(err)
	}

	ifs = GREP(ifs, func(i net.Interface) bool {
		if i.HardwareAddr == nil {
			return false
		}
		return (i.Flags & net.FlagRunning) != 0
	})
	ass := []string{}
	for _, i := range ifs {
		addrs, _ := i.Addrs()
		addrs = GREP(addrs, func(in net.Addr) bool { return v4Re.MatchString(in.String()) })
		ass = append(ass, MAP(addrs, func(in net.Addr) string {
			return v4Re.FindString(in.String())
		})...)
	}

	println("Please connect to:")
	con := app.FromContext(ctx)
	bindparts := strings.SplitN(con.Cfg.BindAddress, ":", 2)
	if len(bindparts) < 2 {
		bindparts[0] = ""
	} else {
		bindparts[0] = ":" + bindparts[1]
	}
	for _, addr := range ass {
		conn := "http://" + addr + bindparts[0] + "/"
		fmt.Println(" => " + conn)
		qrterminal.Generate(conn, qrterminal.M, os.Stdout)
	}
}
