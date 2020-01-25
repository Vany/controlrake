#!/usr/bin/perl

use 5.020;

use Data::Dumper;
my $fname = $ENV{GOFILE};

my @flines = ();
my %blocks = ();
my @codes = ();

my %STASH;

sub readTillEndOfCode {
    my $fh = shift;
    my $buff = "";
    while(my $str = <$fh>) {
        last if $str =~ / \*\/ /x;
        $buff .= $str;
    }
    return $buff;
}


sub rewindTillEndOfBlock {
    my ($fh, $blockName) = @_;
    while(my $str = <$fh>) {
        chomp($str);
        return $str if $str =~ m/ \/\/ \s* END \s+ $blockName/x;
    }
}


sub putblock {
    my ($blockname, $text) = @_;
    $flines[$blocks{$blockname}] = $text;
}


open my $fh, "<", $fname or die "read $fname : $!";
while(my $str = <$fh>) {
    chomp($str);

    if( $str =~ m/ \/ \* \s* PERLCODE /x) {
        my $code = readTillEndOfCode($fh);
        push @flines, $str;
        push @flines, $code, '*/';
        push @codes, $code;

    }
    elsif ($str =~ m/ \/\/ \s* BEGIN \s+ (.+) $ /x) {
        my $blockName = $1;
        push @flines, $str;
        $blocks{$blockName} = 0 + @flines;
        push @flines, "STUB FOR $blockName";
        push @flines, rewindTillEndOfBlock($fh, $blockName);

    } else {
        push @flines, $str

    }
}
close $fh;


for (@codes) {
    eval $_;
    die $@ if $@;
}

open $fh, ">", $fname or die "Can't write to $fname: $!";
print $fh join "\n", @flines;
close $fh;