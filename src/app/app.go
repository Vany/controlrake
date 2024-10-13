package app

import (
	"context"
	"github.com/rs/zerolog"
	"github.com/vany/controlrake/src/config"
	"github.com/vany/controlrake/src/httpserver"
	httpserver_api "github.com/vany/controlrake/src/httpserver/api"
	"github.com/vany/controlrake/src/obs"
	obs_api "github.com/vany/controlrake/src/obs/api"
	"github.com/vany/controlrake/src/obsbrowser"
	obsbrowser_api "github.com/vany/controlrake/src/obsbrowser/api"
	"github.com/vany/controlrake/src/qrcoder"
	"github.com/vany/controlrake/src/widget"
	widget_api "github.com/vany/controlrake/src/widget/api"
	. "github.com/vany/pirog"
	"os"
)

var Key = struct{}{}

// container with all my goodies
type App struct {
	Config     *config.Config            `injectable:"Config"`
	Logger     *zerolog.Logger           `injectable:"Logger"`
	QrCoder    *qrcoder.QrCoder          `injectable:"Qrcoder"`
	ObsBrowser obsbrowser_api.ObsBrowser `injectable:"ObsBrowser"`
	Widgets    widget_api.WidgetRegistry `injectable:"Widgets"`
	Obs        obs_api.Obs               `injectable:"Obs"`
	HTTPServer httpserver_api.HTTPServer `injectable:"HTTPServer"`
}

func New(ctx context.Context) *App {
	app := &App{
		Config:     config.New(),
		Logger:     REF(zerolog.New(os.Stdout)),
		HTTPServer: httpserver.New(),
		Obs:        obs.New(),
		ObsBrowser: obsbrowser.New(),
		Widgets:    widget.NewComponent(),
		QrCoder:    qrcoder.New(),
	}
	InjectComponents(app)

	ExecuteOnAllFields(ctx, app, "Init")

	return app
}
