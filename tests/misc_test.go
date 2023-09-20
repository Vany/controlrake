package tests

import (
	"context"
	"github.com/spf13/viper"
	"github.com/stretchr/testify/assert"
	"github.com/vany/controlrake/src/app"
	"github.com/vany/controlrake/src/config"
	"testing"
)

func Test_Context(t *testing.T) {
	viper.AddConfigPath("../")
	ctx := context.Background()
	ctx = config.ReadConfigToContext(ctx)

	cfg, _ := app.FromContext(ctx)
	assert.True(t, len(cfg.Widgets) > 0)

	//cfg.Widgets = append(cfg.Widgets, types.Widget{})
	//
	//ctx = types.WithValues(ctx, cfg)

}
