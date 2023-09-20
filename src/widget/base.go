// package widget: Widget is inner representation of functionality, that connects web and server part.

package widget

import (
	"bytes"
	"context"
	"fmt"
	"github.com/mitchellh/mapstructure"
	"github.com/vany/controlrake/src/app"
	"github.com/vany/controlrake/src/types"
	"html/template"
	"io"
	"reflect"

	. "github.com/vany/pirog"
)

// all of initialized widgets (from config)
type Registry struct {
	Map   map[string]Widget // Registered widgets
	Order []string          // order which widgets was in config
}

func (r *Registry) Dispatch(ctx context.Context, b []byte) error {
	go func() {
		parts := bytes.SplitN(b, []byte{'|'}, 2)
		name := string(parts[0])

		if w, ok := r.Map[name]; !ok {
			SendChan <- "error|" + fmt.Sprintf("widget %s not found", name)
			return
		} else if err := w.Consume(ctx, parts[1]); err != nil {
			SendChan <- "error|" + w.Base().Errorf("can't consume: %s", err).Error()
		}
	}()
	return nil
}

var SendChan = make(chan string)

func (r *Registry) SendChan() chan string { return SendChan }

func (r *Registry) RenderTo(ctx context.Context, wr io.Writer) error {
	for _, n := range r.Order {
		w := r.Map[n]
		if err := w.RenderTo(wr); err != nil {
			app.FromContext(ctx).Log.Error().Err(err).Msgf("%s render failed", n)
			return w.Base().Errorf("%s render failed", n)
		}
	}
	return nil
}

func NewRegistry(ctx context.Context, confs []map[string]any) types.WidgetRegistry {
	r := Registry{Map: make(map[string]Widget)}
	for _, msa := range confs {
		cfg := Config{}
		mapstructure.Decode(msa, &cfg)
		r.Map[cfg.Name] = New(ctx, cfg)
		r.Order = append(r.Order, cfg.Name)
	}
	return &r
}

var TypeRegistry = make(map[string]reflect.Type)           // typename => widgettype
var TemplateRegistry = make(map[string]*template.Template) // typename => html.template

func RegisterWidgetType(w Widget, tmplstring string) error {
	t := reflect.TypeOf(w).Elem()
	TypeRegistry[t.Name()] = t
	if tmpl, err := template.New(t.Name()).Funcs(map[string]any{
		"UnEscape": func(s string) template.JS { return template.JS(s) },
	}).Parse(
		`<div class="widget" id={{.Name}}>` + tmplstring + `</div>`,
	); err != nil {
		return fmt.Errorf("can't compile html template for %s: %w", t.Name(), err)
	} else {
		TemplateRegistry[t.Name()] = tmpl
	}
	return nil
}

func New(ctx context.Context, cfg Config) Widget {
	if t, ok := TypeRegistry[cfg.Type]; !ok {
		panic("unknown widget type: " + cfg.Type)
	} else {
		w := reflect.New(t).Interface().(Widget)
		w.Base().Config = cfg
		w.Base().Widget = w
		MUST(w.Init(ctx))
		return w
	}
}

type Widget interface {
	Init(ctx context.Context) error                  // init widget with config in it's base
	RenderTo(writer io.Writer) error                 // render visual representation
	Consume(ctx context.Context, event []byte) error // consume one event from Websocket
	Send(event string) error                         // Send something to all my visual representations
	Base() *BaseWidget                               // access to common data
	Actual() Widget                                  // pointer to actual widget
}

type Config struct {
	Name    string // Unique widget id
	Type    string // Type of widget class
	Caption string // Text to render in widget if it is a button or something like this
	Args    any    // Widget specific config
}

type BaseWidget struct {
	Config
	Widget Widget
}

// Consume websocket message in separate goroutine
func (w *BaseWidget) Consume(ctx context.Context, event []byte) error {
	return w.Errorf("Consume() is not implemented")
}

func (w *BaseWidget) RenderTo(wr io.Writer) error {
	if tmpl, ok := TemplateRegistry[w.Type]; !ok {
		_, err := wr.Write([]byte(w.Name + " => " + w.Type))
		return err
	} else {
		return tmpl.Execute(wr, w.Actual())
	}
}

func (w *BaseWidget) Init(context.Context) error {
	return nil
}

func (w *BaseWidget) Base() *BaseWidget {
	return w
}

func (w *BaseWidget) Actual() Widget {
	return w.Widget
}

func (w *BaseWidget) Send(msg string) error {
	SendChan <- w.Name + "|" + msg
	return nil
}

func (w *BaseWidget) Errorf(f string, args ...any) error {
	return fmt.Errorf("name: %s, type: %s "+f, w.Name, w.Type, args)
}

func MustSurvive(err error) struct{} {
	MUST(err)
	return struct{}{}
}
