// LICENSE APACHE 2.0
// (C) Vany Serezhkin 2020

package main

import (
	"context"
	"github.com/vany/controlrake/src/config"
	"github.com/vany/controlrake/src/cont"
	"github.com/vany/controlrake/src/http"
	"github.com/vany/controlrake/src/types"
	"github.com/vany/controlrake/src/widget"
	. "github.com/vany/pirog"
	"os"
	"os/signal"
)

func main() {

	ctx, cf := signal.NotifyContext(context.Background(), os.Interrupt)
	defer cf()
	ctx = cont.PutToContext(ctx, MUST2(config.ReadConfig(ctx)))
	ctx = cont.PutToContext(ctx, types.NewLogger())
	con := cont.FromContext(ctx)
	ctx = cont.PutToContext(ctx, widget.NewRegistry(ctx, con.Cfg.Widgets))
	// components.serve()

	MUST(http.ListenAndServe(ctx))

}
