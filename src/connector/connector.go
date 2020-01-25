package connector

//go:generate perl ../../generate.pl

import (
	"encoding/json"
	"errors"
	"github.com/Vany/controlrake/src/types"

	// BEGIN imports
	"github.com/Vany/controlrake/src/connector/ffmpeg"
	"github.com/Vany/controlrake/src/connector/obs"
	"github.com/Vany/controlrake/src/connector/robotgo"
	//END imports
)

var reg = make(map[string]types.Connector)

func init() {
	//BEGIN init
	reg["ffmpeg"] = ffmpeg.New().Init()
	reg["obs"] = obs.New().Init()
	reg["robotgo"] = robotgo.New().Init()
	//END init
}

func Handle(module string, method string, arg json.RawMessage) error {
	var ca interface{}

	mod, ok := reg[module]
	if !ok {
		return errors.New("There is no module " + module)
	}

	json.Unmarshal(arg, &ca)
	mod.Handle(method, ca)

	return nil
}

/*PERLCODE

my @dirs = ();

opendir(my $dh, ".") || die "Can't open current dir: $!";
while (my $f = readdir $dh) {
	next if $f=~/^\./ or  ! -d $f ;
	push @dirs, $f;
}
closedir $dh;

putblock(imports => join("\n", map {
	"    \"github.com/Vany/controlrake/src/connector/$_\""
} @dirs));

putblock(init => join("\n", map {
	"    reg[\"$_\"] = $_.New().Init()"
} @dirs));




*/
