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

// App - Initialized system components
type App struct {
	Config          *config.ConfigComponent    `injectable:"Config"`
	Logger          *zerolog.Logger            `injectable:"Logger"`
	QrCoder         *qrcoder.QrCoder           `injectable:"Qrcoder"`
	ObsBrowser      obsbrowser_api.ObsBrowser  `injectable:"ObsBrowser"`
	WidgetComponent widget_api.WidgetComponent `injectable:"WidgetComponent"`
	Obs             obs_api.Obs                `injectable:"Obs"`
	HTTPServer      httpserver_api.HTTPServer  `injectable:"HTTPServer"`
}

func New(ctx context.Context) *App {
	app := &App{
		Config:          config.New(),
		Logger:          REF(zerolog.New(os.Stdout).Level(TERNARY(DEBUG, zerolog.DebugLevel, zerolog.InfoLevel))),
		HTTPServer:      httpserver.New(),
		Obs:             obs.New(),
		ObsBrowser:      obsbrowser.New(),
		WidgetComponent: widget.NewComponent(),
		QrCoder:         qrcoder.New(),
	}
	InjectComponents(app)

	if err := ExecuteOnAllFields(ctx, app, "Init"); err != nil {
		return nil
	}
	return app
}
