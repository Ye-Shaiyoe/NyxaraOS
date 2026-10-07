#ifndef NYXARA_HAL_ATA_H
#define NYXARA_HAL_ATA_H

#include "types.h"

#define ATA_PRIMARY_IO     0x1F0
#define ATA_PRIMARY_CTRL   0x3F6

#define ATA_REG_DATA       0x1F0
#define ATA_REG_ERROR      0x1F1
#define ATA_REG_SECCOUNT   0x1F2
#define ATA_REG_LBA_LO     0x1F3
#define ATA_REG_LBA_MID    0x1F4
#define ATA_REG_LBA_HI     0x1F5
#define ATA_REG_DRIVE      0x1F6
#define ATA_REG_COMMAND    0x1F7
#define ATA_REG_STATUS     0x1F7

#define ATA_CMD_READ_PIO   0x20
#define ATA_CMD_WRITE_PIO  0x30
#define ATA_CMD_IDENTIFY   0xEC
#define ATA_CMD_FLUSH      0xE7

#define ATA_SR_BSY  0x80
#define ATA_SR_DRDY 0x40
#define ATA_SR_DRQ  0x08
#define ATA_SR_ERR  0x01

int  ata_identify(void);
int  ata_read_sectors(uint32_t lba, uint8_t count, uint8_t *buf);
int  ata_write_sectors(uint32_t lba, uint8_t count, const uint8_t *buf);

#endif
