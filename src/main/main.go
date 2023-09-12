// LICENSE APACHE 2.0
// (C) Vany Serezhkin 2020

package main

import (
	"context"
	"github.com/vany/controlrake/src/config"
	"github.com/vany/controlrake/src/http"
	"github.com/vany/controlrake/src/types"
	"github.com/vany/controlrake/src/widget"
	. "github.com/vany/pirog"
)

func main() {
	ctx := context.Background()
	ctx = types.PutToContext(ctx, MUST2(config.ReadConfig(ctx)))
	ctx = types.CreateLoggerToContext(ctx)
	con := types.FromContext(ctx)
	ctx = types.PutToContext(ctx, widget.NewRegistry(ctx, con.Cfg.Widgets))
	// components.serve()

	MUST(http.ListenAndServe(ctx))
}
