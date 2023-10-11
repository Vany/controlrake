package app

import (
	"context"
	"fmt"
	"github.com/vany/controlrake/src/config"
	"github.com/vany/controlrake/src/types"
	"reflect"
)

var Key = struct{}{}

// container with all my goodies
type App struct {
	Cfg        *config.Config
	Log        *types.Logger
	Widget     types.WidgetRegistry
	Obs        types.Obs
	ObsBrowser types.ObsBrowser
	HTTP       types.HTTPServer
	Youtube    types.Youtube
}

func PutToApp(ctx context.Context, obj any) context.Context {
	c := FromContext(ctx)
	if c == nil {
		c = &App{}
		ctx = context.WithValue(ctx, Key, c)
	}

	ft := reflect.TypeOf(c).Elem()
	ot := reflect.TypeOf(obj)
	for i := 0; i < ft.NumField(); i++ {
		ftc := ft.Field(i).Type
		if ftc.Kind() == reflect.Pointer {
			ftc = ftc.Elem()
		}
		if ot.Elem().AssignableTo(ftc) || (ftc.Kind() == reflect.Interface && ot.Implements(ftc)) {
			reflect.ValueOf(c).Elem().Field(i).Set(reflect.ValueOf(obj))
		}
	}
	return ctx
}

// only on interface fields
func (a *App) ExecuteInitStage(ctx context.Context, stage int) error {
	v := reflect.ValueOf(a).Elem()
	for i := 0; i < v.NumField(); i++ {
		f := v.Field(i).Elem()
		m := f.MethodByName(fmt.Sprintf("InitStage%d", stage))
		if !m.IsValid() {
			continue
		}
		ret := m.Call([]reflect.Value{reflect.ValueOf(ctx)})[0]
		if !ret.IsNil() {
			return ret.Interface().(error)
		}
	}
	return nil
}

func FromContext(ctx context.Context) *App {
	c := ctx.Value(Key) // or die
	if c == nil {
		return nil
	} else {
		return c.(*App)
	}
}
