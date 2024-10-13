// LICENSE APACHE 2.0
// (C) Vany Serezhkin 2024

package main

import (
	"context"
	"github.com/vany/controlrake/src/app"
	. "github.com/vany/pirog"
	"os"
	"os/signal"
)

func main() {

	ctx, cf := signal.NotifyContext(context.Background(), os.Interrupt)
	defer cf()

	a := app.New(ctx)
	ExecuteOnAllFields(ctx, a, "Run")

	a.QrCoder.DrawIps()

	<-ctx.Done()
	ExecuteOnAllFields(ctx, a, "Stop")

}
