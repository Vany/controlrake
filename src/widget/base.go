// package widget: Widget is inner representation of functionality, that connects web and server part.
package widget

import (
	"bytes"
	"context"
	"fmt"
	"github.com/mitchellh/mapstructure"
	"github.com/rs/zerolog"
	app2 "github.com/vany/controlrake/src/app"
	. "github.com/vany/pirog"
	"io"
	"reflect"
	"text/template"
)

var TypeRegistry = make(map[string]reflect.Type)           // typename => widgettype
var TemplateRegistry = make(map[string]*template.Template) // typename => html.template

type RawHTML string

func (r RawHTML) String() string { return string(r) } // template.JS

func RegisterWidgetType(w Widget, tmplstring string) error {
	t := reflect.TypeOf(w).Elem()
	TypeRegistry[t.Name()] = t
	if tmpl, err := template.New(t.Name()).Funcs(map[string]any{
		"WriteBuffer": func() io.Writer { return &bytes.Buffer{} },
	}).Parse(
		`<div class="widget" id="{{.Name}}" {{if .Style}}style="{{.Style}}"{{end}}>` + tmplstring + `</div>`,
	); err != nil {
		return fmt.Errorf("can't compile html template for %s: %w", t.Name(), err)
	} else {
		TemplateRegistry[t.Name()] = tmpl
	}
	return nil
}

func New(ctx context.Context, cfga any) Widget {
	cfg := Config{}
	app := app2.FromContext(ctx)
	mapstructure.Decode(cfga, &cfg)
	if t, ok := TypeRegistry[cfg.Type]; !ok {
		panic("unknown widget type: " + cfg.Type)
	} else {
		w := reflect.New(t).Interface().(Widget)
		w.Base().Config = cfg
		w.Base().Widget = w
		w.Base().Log = app.Log.With().Str("widget", cfg.Name).Logger()
		return w
	}
}

func (w *BaseWidget) InitStage1(ctx context.Context) error {
	if w.Chan == nil {
		w.Chan = make(chan string)
	}
	w.Widget.Init(ctx)

	InitChildren(ctx, w.Widget)
	return nil
}

// TODO may be VisitChildren()
func InitChildren(ctx context.Context, w Widget) error {
	for n, nw := range w.Children() {
		if err := nw.Init(ctx); err != nil {
			return err
		} else if err := InitChildren(ctx, nw); err != nil {
			return fmt.Errorf("%s : %w", n, err)
		}
	}
	return nil
}

type Widget interface {
	Init(ctx context.Context) error                   // init widget with config in it's base
	Dispatch(ctx context.Context, event string) error // consume one event from Websocket
	RenderTo(ctx context.Context, wr io.Writer) error // render visual representation
	SendChan() chan string                            // get channel where out messages is
	Send(event string) error                          // Send something to all my visual representations
	Base() *BaseWidget                                // access to common data
	Children() map[string]Widget                      // get all children
}

type Config struct {
	Name    string // Unique widget id
	Type    string // Type of widget class
	Caption string // Text to render in widget if it is a button or something like this
	Style   string // css style for this widget only
	Args    any    // Widget specific config
}

type BaseWidget struct {
	Config
	Widget Widget         // link to actual widget object
	Chan   chan string    // channel to interact with visual representation
	Log    zerolog.Logger // logger for specified widget
}

func (w *BaseWidget) Init(ctx context.Context) error { return nil }
func (w *BaseWidget) Base() *BaseWidget              { return w }
func (w *BaseWidget) SendChan() chan string          { return w.Chan }
func (w *BaseWidget) Children() map[string]Widget    { return nil }

// Consume websocket message in separate goroutine
func (w *BaseWidget) Dispatch(ctx context.Context, event string) error {
	return w.Errorf("Dispatch() is not implemented")
}

func (w *BaseWidget) RenderTo(ctx context.Context, wr io.Writer) error {
	if tmpl, ok := TemplateRegistry[w.Type]; !ok {
		_, err := wr.Write([]byte(w.Name + " => " + w.Type))
		return err
	} else {
		return tmpl.Execute(wr, w.Widget)
	}
}

func (w *BaseWidget) Send(msg string) error {
	w.Chan <- w.Name + "|" + msg
	return nil
}

func (w *BaseWidget) Errorf(f string, args ...any) error {
	return fmt.Errorf("%s(%s)"+f, append([]any{w.Type, w.Name}, args...)...)
}

func MustSurvive(err error) struct{} {
	MUST(err)
	return struct{}{}
}
