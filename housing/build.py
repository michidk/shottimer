"""Build, validate and export one of the current housings."""
import argparse


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('model',choices=('stand','magnetic'))
    parser.add_argument('--usb',choices=('left','right','up'))
    parser.add_argument('--magnet',choices=('left','right'))
    args=parser.parse_args()
    if args.model=='stand':
        if args.magnet or args.usb not in (None,'right'):
            parser.error('The desktop stand currently has right-side USB and no magnet.')
        from cad.stand import main as build
        build()
    else:
        from cad.magnetic import main as build
        build(args.usb or 'left',args.magnet or 'right')


if __name__=='__main__':
    main()
