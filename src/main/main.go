// LICENSE APACHE 2.0
// (C) Vany Serezhkin 2020

package main

import (
	"context"
	"fmt"
	"github.com/mdp/qrterminal/v3"
	"github.com/vany/controlrake/src/app"
	"github.com/vany/controlrake/src/config"
	"github.com/vany/controlrake/src/httpserver"
	"github.com/vany/controlrake/src/obs"
	"github.com/vany/controlrake/src/obsbrowser"
	"github.com/vany/controlrake/src/types"
	"github.com/vany/controlrake/src/widget"
	"github.com/vany/controlrake/src/youtube"
	. "github.com/vany/pirog"
	"net"
	"os"
	"os/signal"
	"regexp"
)

func main() {

	ctx, cf := signal.NotifyContext(context.Background(), os.Interrupt)
	defer cf()
	ctx = app.PutToApp(ctx, MUST2(config.ReadConfig(ctx)))

	ctx = app.PutToApp(ctx, MUST2(httpserver.New(ctx)))
	ctx = app.PutToApp(ctx, types.NewLogger())
	con := app.FromContext(ctx)
	ctx = app.PutToApp(ctx, widget.New(ctx, con.Cfg.Widget))
	ctx = app.PutToApp(ctx, obs.New(ctx))
	ctx = app.PutToApp(ctx, obsbrowser.New(ctx))
	ctx = app.PutToApp(ctx, MUST2(youtube.New(ctx)))

	MUST(con.ExecuteInitStage(ctx, 1))

	GetMyAddrs(ctx)

	<-ctx.Done()
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
	app := app.FromContext(ctx)
	for _, addr := range ass {
		conn := app.HTTP.GetBaseUrl(addr)
		fmt.Println(" => " + conn)
		qrterminal.Generate(conn, qrterminal.M, os.Stdout)
	}
}
