// package widget: Widget is inner representation of functionality, that connects web and server part.
package widget

import (
	"context"
	"fmt"
	"github.com/mitchellh/mapstructure"
	"html/template"
	"io"
	"reflect"

	. "github.com/vany/pirog"
)

var TypeRegistry = make(map[string]reflect.Type)           // typename => widgettype
var TemplateRegistry = make(map[string]*template.Template) // typename => html.template

func RegisterWidgetType(w Widget, tmplstring string) error {
	t := reflect.TypeOf(w).Elem()
	TypeRegistry[t.Name()] = t
	if tmpl, err := template.New(t.Name()).Funcs(map[string]any{
		"UnEscape": func(s string) template.JS { return template.JS(s) },
	}).Parse(
		`<div class="widget" id={{.Name}} {{if .Style}}style="{{.Style}}"{{end}}>` + tmplstring + `</div>`,
		//`<div class="widget" id={{.Name}} >` + tmplstring + `</div>`,
	); err != nil {
		return fmt.Errorf("can't compile html template for %s: %w", t.Name(), err)
	} else {
		TemplateRegistry[t.Name()] = tmpl
	}
	return nil
}

func New(ctx context.Context, cfga any) Widget {
	cfg := Config{}
	mapstructure.Decode(cfga, &cfg)
	if t, ok := TypeRegistry[cfg.Type]; !ok {
		panic("unknown widget type: " + cfg.Type)
	} else {
		w := reflect.New(t).Interface().(Widget)
		w.Base().Config = cfg
		w.Base().Widget = w
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
	RenderTo(ctx context.Context, wr io.Writer) error // render visual representation
	Dispatch(ctx context.Context, event []byte) error // consume one event from Websocket
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
	Widget Widget      // link to actual widget object
	Chan   chan string // channel to interact with visual representation
}

// Consume websocket message in separate goroutine
func (w *BaseWidget) Dispatch(ctx context.Context, event []byte) error {
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

func (w *BaseWidget) Init(ctx context.Context) error {
	return nil
}

func (w *BaseWidget) Base() *BaseWidget {
	return w
}

func (w *BaseWidget) SendChan() chan string { return w.Chan }

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

func (w *BaseWidget) Children() map[string]Widget {
	return nil
}
